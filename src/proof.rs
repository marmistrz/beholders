use itertools::izip;
use kzg_traits::{EcBackend, Fr, G1Mul, KZGSettings, G1};

use crate::check;
use crate::commitment::{get_point, open_all, Opening};
use crate::hashing::{derive_indices, individual_hash, pow_pass, prelude, HashOutput, Prelude};
use crate::schnorr::{PublicKey, Schnorr, SecretKey};
use crate::util::bitxor;

type Commitment<B> = <B as EcBackend>::G1;

const BYTE_DIFFICULTY: usize = 1;
const MAXC: u64 = u16::MAX as u64;

// TODO include beacon
pub struct BaseProof<B: EcBackend, const M: usize> {
    schnorr: Schnorr<B>, // (a, c, z)
    data: [u64; M],
    openings: [Opening<B>; M],
}

pub struct Proof<B: EcBackend, const M: usize> {
    pub base_proofs: Vec<BaseProof<B, M>>,
}

impl<B: EcBackend, const M: usize> Proof<B, M> {
    fn prelude(&self) -> Prelude {
        // FIXME: we should hash more than just the a_i's
        let a_i = self.base_proofs.iter().map(|x| x.schnorr.a.clone());
        prelude(a_i)
    }

    pub fn verify(
        &self,
        pk: &PublicKey<B>,
        com: &Commitment<B>,
        data_len: usize,
        kzg_settings: &B::KZGSettings,
    ) -> Result<bool, String> {
        let prelude = self.prelude();
        for (i, base_proof) in self.base_proofs.iter().enumerate() {
            if !base_proof.verify(i, prelude, pk, com, data_len, kzg_settings)? {
                println!("Failed at base proof {}", i);
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub fn prove(
        kzg_settings: &B::KZGSettings,
        sk: SecretKey<B>,
        data: &[u64],
        nfisch: usize,
    ) -> Result<Option<Self>, String> {
        let generator = B::G1::generator();
        // Compute the openings
        let openings: Vec<Opening<B>> = open_all::<B>(kzg_settings, data)?;

        // Compute the Schnorr commitment
        let r_i: Vec<_> = (0..nfisch).map(|_| B::Fr::rand()).collect();
        let a_i = r_i.iter().map(|r| generator.mul(r));

        // TODO: prelude should contain more
        let prelude = prelude(a_i);

        let proofs: Option<Vec<_>> = (0..nfisch)
            .map(|fisch_iter| {
                BaseProof::<B, M>::prove(
                    fisch_iter,
                    prelude,
                    &openings,
                    &r_i[fisch_iter],
                    &sk,
                    data,
                )
            })
            .collect();
        Ok(proofs.map(|base_proofs| Self { base_proofs }))
    }
}

impl<B: EcBackend, const M: usize> BaseProof<B, M> {
    fn verify(
        &self,
        fisch_iter: usize,
        prelude: Prelude,
        pk: &PublicKey<B>,
        com: &Commitment<B>,
        data_len: usize,
        kzg_settings: &B::KZGSettings,
    ) -> Result<bool, String> {
        let fft_settings = kzg_settings.get_fft_settings();

        println!("Checking Schnorr");
        check!(self.schnorr.verify(pk));

        // Compute the indices
        let indices = derive_indices(fisch_iter, &self.schnorr.c, M, data_len);
        let indices: [usize; 8] = indices.try_into().expect("FIXME support m != 8");

        let mut hash = HashOutput::default();

        // Verify openings and accumulate PoW
        for ((k, idx), value, opening) in
            izip!(indices.into_iter().enumerate(), self.data, &self.openings)
        {
            let k = k.try_into().unwrap();
            let val = B::Fr::from_u64(value);
            let x = get_point(fft_settings, data_len, idx);

            check!(kzg_settings.check_proof_single(com, opening, x, &val)?);

            let partial_pow = individual_hash(prelude, &self.schnorr, k, value, opening);
            hash = bitxor(hash, partial_pow);
        }

        // Check PoW
        check!(pow_pass(&hash, BYTE_DIFFICULTY));

        Ok(true)
    }

    fn prove(
        fisch_iter: usize,
        prelude: Prelude,
        openings: &[Opening<B>],
        r: &B::Fr,
        sk: &SecretKey<B>,
        data: &[u64],
    ) -> Option<Self> {
        for c in 0..MAXC {
            // TODO check if direct add is faster
            let c = B::Fr::from_u64(c);
            let schnorr = Schnorr::<B>::prove(sk, r, c.clone());

            let indices = derive_indices(fisch_iter, &c, M, data.len());
            let indices: [usize; 8] = indices.try_into().expect("FIXME support m != 8");
            let data: Vec<_> = indices.iter().map(|&i| data[i]).collect();
            let openings: Vec<_> = indices.iter().map(|&i| &openings[i]).collect();

            let mut hash = HashOutput::default();
            for (k, (val, opening)) in izip!(data.iter(), openings.iter()).enumerate() {
                let k = k.try_into().unwrap();
                let partial_pow = individual_hash(prelude, &schnorr, k, *val, *opening);

                hash = bitxor(hash, partial_pow);
            }
            if pow_pass(&hash, BYTE_DIFFICULTY) {
                let openings: Vec<_> = openings.into_iter().cloned().collect();
                return Some(BaseProof {
                    schnorr,
                    data: data.try_into().unwrap(),
                    openings: openings.try_into().unwrap(), // [B::G1::zero(); 8],
                });
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use kzg::{
        eip_7594::BlstBackend,
        types::{fft_settings::FsFFTSettings, fr::FsFr, kzg_settings::FsKZGSettings},
        utils::generate_trusted_setup,
    };
    use kzg_traits::FFTSettings;

    use crate::commitment::interpolate;

    use super::*;
    type Backend = BlstBackend;
    const M: usize = 8;

    #[test]
    fn test_base_proof() {
        //, 5, 1, 5, 7];
        let data = [4, 2137, 383, 4];

        let secrets_len = 15;
        let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0; 32]);
        let fs = FsFFTSettings::new(4).unwrap();
        let kzg_settings: FsKZGSettings = FsKZGSettings::new(&s1, &s2, &s3, &fs, 7).unwrap();

        let openings = open_all::<Backend>(&kzg_settings, &data).expect("openings");
        assert_eq!(openings.len(), data.len());

        let g = <Backend as EcBackend>::G1::generator();
        let r = FsFr::from_u64(1337);
        let sk = SecretKey::<Backend>::from_u64(2137);
        let pk = g.mul(&sk);

        let fisch_iter = 0;
        let prelude = [0; 8];

        let proof = BaseProof::<Backend, M>::prove(fisch_iter, prelude, &openings, &r, &sk, &data)
            .expect("No proof found");

        let poly = interpolate(kzg_settings.get_fft_settings(), &data);
        let com = kzg_settings.commit_to_poly(&poly).expect("commit");
        assert!(proof
            .verify(fisch_iter, prelude, &pk, &com, data.len(), &kzg_settings)
            .expect("KZG error"));
    }

    #[test]
    fn test_mining_works() {
        let data = [4, 2137, 383, 4]; //, 5, 1, 5, 7];

        let secrets_len = 15;
        let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0; 32]);
        let fs = FsFFTSettings::new(4).unwrap();
        let kzg_settings: FsKZGSettings = FsKZGSettings::new(&s1, &s2, &s3, &fs, 7).unwrap();

        let g = <Backend as EcBackend>::G1::generator();
        let sk = SecretKey::<Backend>::from_u64(2137);
        let pk = g.mul(&sk);

        let nfisch = 2;
        let proof = Proof::<Backend, M>::prove(&kzg_settings, sk, &data, nfisch)
            .expect("KZG error")
            .expect("No proof found");
        assert_eq!(proof.base_proofs.len(), nfisch);
        for base_proof in &proof.base_proofs {
            assert_eq!(base_proof.data.len(), M);
            assert!(base_proof.schnorr.verify(&pk));
        }

        let poly = interpolate(kzg_settings.get_fft_settings(), &data);
        let com = kzg_settings.commit_to_poly(&poly).expect("commit");
        assert!(proof
            .verify(&pk, &com, data.len(), &kzg_settings)
            .expect("KZG error"));
    }
}

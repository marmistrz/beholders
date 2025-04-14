use itertools::izip;
use kzg_traits::{Fr, G1Mul, KZGSettings, G1};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::check;
use crate::commitment::{get_point, open_all_fk20, Commitment, Opening};
use crate::hashing::{derive_indices, individual_hash, pow_pass, prelude, HashOutput, Prelude};
use crate::schnorr::{PublicKey, Schnorr, SecretKey};
use crate::types::{TFr, TKZGSettings, TG1};
use crate::util::bitxor;

// TODO include beacon
/// A single Fischlin iteration of the beholder signature
#[derive(Debug)]
pub struct BaseProof {
    schnorr: Schnorr, // (a, c, z)
    data: Vec<TFr>,
    openings: Vec<Opening>,
}

/// A complete beholder signature
#[derive(Debug)]
pub struct Proof {
    pub base_proofs: Vec<BaseProof>,
}

impl Proof {
    fn prelude(&self, pk: &PublicKey, com: &Commitment) -> Prelude {
        let a_i = self.base_proofs.iter().map(|x| x.schnorr.a);
        prelude(pk, com, a_i)
    }

    /// Verifies the Beholder Signature.
    ///
    /// # Arguments
    ///
    /// * `pk` - Schnorr public key.
    /// * `com` - KZG commitment for the data.
    /// * `data_len` - Length of the underlying data.
    /// * `kzg_settings` - KZG trusted setup.
    /// * `difficulty` - The bit difficulty, i.e., the required number of leading zeros.
    ///
    /// # Returns
    ///
    /// An error is return in case of a KZG error. Otherwise, returns `true` if the verification is successful.
    pub fn verify(
        &self,
        pk: &PublicKey,
        com: &Commitment,
        data_len: usize,
        kzg_settings: &TKZGSettings,
        difficulty: u32,
        mvalue: usize,
    ) -> Result<bool, String> {
        let prelude = self.prelude(pk, com);
        for (i, base_proof) in self.base_proofs.iter().enumerate() {
            if !base_proof.verify(
                i,
                prelude,
                pk,
                com,
                data_len,
                kzg_settings,
                difficulty,
                mvalue,
            )? {
                println!("Failed at base proof {}", i);
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Generates a Beholder Signature for the given data.
    ///
    /// # Arguments
    ///
    /// * `kzg_settings` - KZG trusted setup.
    /// * `sk` - Schnorr secret key.
    /// * `data` - The data to be proven. The length of the data must be a power of two and is assumed to be error-corrected.
    /// * `nfisch` - Number of Fischlin proofs to generate.
    /// * `difficulty` - The bit difficulty, i.e., the required number of leading zeros.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(Self))` if the proof generation is successful, `Ok(None)` if it fails,
    /// or an `Err` with a string message in case of an error.
    pub fn prove(
        kzg_settings: &TKZGSettings,
        sk: SecretKey,
        data: &[u8],
        nfisch: usize,
        difficulty: u32,
        mvalue: usize,
    ) -> Result<Option<Self>, String> {
        assert!(
            data.len().is_power_of_two(),
            "Data length must be a power of two"
        );

        let data: Vec<_> = data
            .chunks_exact(32)
            .map(|x| TFr::from_bytes_unchecked(x).unwrap())
            .collect();

        let generator = TG1::generator();
        // Compute the openings-
        let (com, openings) = open_all_fk20(kzg_settings, &data)?;

        // Compute the Schnorr commitment
        let r_i: Vec<_> = (0..nfisch).map(|_| TFr::rand()).collect();
        let a_i = r_i.iter().map(|r| generator.mul(r));

        let pk = generator.mul(&sk);
        let prelude = prelude(&pk, &com, a_i);

        let proofs: Option<Vec<_>> = (0..nfisch)
            .into_par_iter()
            .map(|fisch_iter| {
                BaseProof::prove(
                    fisch_iter,
                    prelude,
                    &openings,
                    &r_i[fisch_iter],
                    &sk,
                    &data,
                    difficulty,
                    mvalue,
                )
            })
            .collect();
        Ok(proofs.map(|base_proofs| Self { base_proofs }))
    }
}

impl BaseProof {
    fn verify(
        &self,
        fisch_iter: usize,
        prelude: Prelude,
        pk: &PublicKey,
        com: &Commitment,
        data_len: usize,
        kzg_settings: &TKZGSettings,
        difficulty: u32,
        mvalue: usize,
    ) -> Result<bool, String> {
        let fft_settings = kzg_settings.get_fft_settings();

        println!("Checking Schnorr");
        check!(self.schnorr.verify(pk));

        // Compute the indices as a Vec<usize>
        let indices: Vec<usize> = derive_indices(fisch_iter, self.schnorr.c, mvalue, data_len);
        // Ensure that we have the correct number of indices.
        assert_eq!(indices.len(), mvalue);

        // Compute the indices
        //let indices = derive_indices(fisch_iter, &self.schnorr.c, mvalue, data_len);
        //let indices: [usize; mvalue] = indices.try_into().expect("invalid num_indices");

        let mut hash = HashOutput::default();

        assert_eq!(self.data.len(), self.openings.len());
        // Verify openings and accumulate PoW
        for ((k, idx), value, opening) in
            izip!(indices.into_iter().enumerate(), &self.data, &self.openings)
        {
            let k = k.try_into().unwrap();
            let x = get_point(fft_settings, data_len, idx);

            check!(kzg_settings.check_proof_single(com, opening, x, value)?);

            let partial_pow =
                individual_hash(prelude, &self.schnorr, fisch_iter, k, *value, opening);
            hash = bitxor(hash, partial_pow);
        }

        // Check PoW
        check!(pow_pass(&hash, difficulty));

        Ok(true)
    }

    pub fn prove(
        fisch_iter: usize,
        prelude: Prelude,
        openings: &[Opening],
        r: &TFr,
        sk: &SecretKey,
        data: &[TFr],
        difficulty: u32,
        mvalue: usize,
    ) -> Option<Self> {
        assert_eq!(data.len(), openings.len());
        let maxc = 1u32 << (difficulty + 5);
        for c in 0..maxc {
            let schnorr = Schnorr::prove(sk, r, c);

            let indices = derive_indices(fisch_iter, c, mvalue, data.len());
            let indices: [usize; 16] = indices.try_into().expect("FIXME support m != 16");
            let data: Vec<_> = indices.iter().map(|&i| data[i]).collect();
            let openings: Vec<_> = indices.iter().map(|&i| &openings[i]).collect();

            let mut hash = HashOutput::default();
            for (k, (val, opening)) in izip!(data.iter(), openings.iter()).enumerate() {
                let k = k.try_into().unwrap();
                let partial_pow = individual_hash(prelude, &schnorr, fisch_iter, k, *val, *opening);

                hash = bitxor(hash, partial_pow);
            }
            if pow_pass(&hash, difficulty) {
                let openings: Vec<_> = openings.into_iter().copied().collect();
                return Some(BaseProof {
                    schnorr,
                    data,
                    openings,
                });
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use kzg::{
        types::{fft_settings::FsFFTSettings, fr::FsFr, kzg_settings::FsKZGSettings},
        utils::generate_trusted_setup,
    };
    use kzg_traits::FFTSettings;

    use crate::commitment::interpolate;

    use super::*;
    const M: usize = 16;

    #[test]
    fn test_base_proof() {
        let data: Vec<TFr> = vec![4, 2137, 383, 4]
            .into_iter()
            .map(TFr::from_u64)
            .collect();

        let secrets_len = 15;
        let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0; 32]);
        let fs = FsFFTSettings::new(4).unwrap();
        let kzg_settings: FsKZGSettings = FsKZGSettings::new(&s1, &s2, &s3, &fs, 7).unwrap();

        let (_com, openings) = open_all_fk20(&kzg_settings, &data).expect("openings");
        assert_eq!(openings.len(), data.len());

        let g = TG1::generator();
        let r = FsFr::from_u64(1337);
        let sk = SecretKey::from_u64(2137);
        let pk = g.mul(&sk);
        let byte_difficulty = 4;

        let fisch_iter = 0;
        let prelude = [0; 8];
        let mvalue: usize = 16;

        let proof = BaseProof::prove(
            fisch_iter,
            prelude,
            &openings,
            &r,
            &sk,
            &data,
            byte_difficulty,
            mvalue,
        )
        .expect("No proof found");

        let poly = interpolate(kzg_settings.get_fft_settings(), &data);
        let com = kzg_settings.commit_to_poly(&poly).expect("commit");
        assert!(proof
            .verify(
                fisch_iter,
                prelude,
                &pk,
                &com,
                data.len(),
                &kzg_settings,
                byte_difficulty,
                mvalue
            )
            .expect("KZG error"));
    }

    #[test]
    fn test_mining_works() {
        let data = [4; 128]; //, 5, 1, 5, 7];
        let bit_difficulty = 1;

        let secrets_len = 15;
        let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0; 32]);
        let fs = FsFFTSettings::new(4).unwrap();
        let kzg_settings: FsKZGSettings = FsKZGSettings::new(&s1, &s2, &s3, &fs, 7).unwrap();

        let g = TG1::generator();
        let sk = SecretKey::from_u64(2137);
        let pk = g.mul(&sk);

        let nfisch = 2;
        let mvalue: usize = 16;
        let proof = Proof::prove(&kzg_settings, sk, &data, nfisch, bit_difficulty, mvalue)
            .expect("KZG error")
            .expect("No proof found");
        assert_eq!(proof.base_proofs.len(), nfisch);
        for base_proof in &proof.base_proofs {
            assert_eq!(base_proof.data.len(), M);
            assert!(base_proof.schnorr.verify(&pk));
        }

        let data: Vec<_> = data
            .chunks_exact(32)
            .map(|x| TFr::from_bytes_unchecked(x).unwrap())
            .collect();
        let poly = interpolate(kzg_settings.get_fft_settings(), &data);
        let com = kzg_settings.commit_to_poly(&poly).expect("commit");
        assert!(proof
            .verify(&pk, &com, data.len(), &kzg_settings, bit_difficulty, mvalue)
            .expect("KZG error"));
    }
}

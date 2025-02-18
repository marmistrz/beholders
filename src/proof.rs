use itertools::izip;
use kzg_traits::{EcBackend, Fr, G1Mul, KZGSettings, G1};

use crate::check;
use crate::commitment::get_point;
use crate::hashing::{derive_indices, individual_hash, pow_pass, prelude, HashOutput, Prelude};
use crate::schnorr::{PublicKey, Schnorr, SecretKey};
use crate::util::bitxor;

type Opening<B: EcBackend> = B::G1;
type Commitment<B: EcBackend> = B::G1;

const BYTE_DIFFICULTY: usize = 2;
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

impl<B: EcBackend, const NFISCH: usize> Proof<B, NFISCH> {
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
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn prove(sk: SecretKey<B>, data: &[u64]) -> Option<Self> {
        let generator = B::G1::generator();
        // Compute the openings
        let openings: Vec<Opening<B>> = Vec::new();
        // TODO

        // Compute the Schnorr commitment
        let r_i: Vec<_> = (0..NFISCH).map(|_| B::Fr::rand()).collect();
        let a_i = r_i.iter().map(|r| generator.mul(r));

        // TODO: prelude should contain more
        let prelude = prelude(a_i);

        let proofs: Option<Vec<_>> = (0..NFISCH)
            .map(|fisch_iter| {
                BaseProof::<B, NFISCH>::prove(
                    fisch_iter,
                    prelude,
                    &openings,
                    &r_i[fisch_iter],
                    &sk,
                    data,
                )
            })
            .collect();
        proofs.map(|base_proofs| Self { base_proofs })
    }
}

impl<B: EcBackend, const NFISCH: usize> BaseProof<B, NFISCH> {
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

        check!(self.schnorr.verify(pk));

        // Compute the indices
        let indices = derive_indices(fisch_iter, &self.schnorr.c, 8);
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

            // FIXME compute hash properly
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

            let indices = derive_indices(fisch_iter, &c, 8);
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
    use kzg::eip_7594::BlstBackend;

    use super::*;
    type Backend = BlstBackend;

    // #[test]
    // fn test_mining_works() {
    //     let baseproof = BaseProof::<Backend>::prove();
    // }
}

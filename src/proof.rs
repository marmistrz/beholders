use itertools::izip;
use kzg_traits::{EcBackend, Fr, KZGSettings, G1};

use crate::check;
use crate::commitment::get_point;
use crate::hashing::{derive_indices, mine, pow_pass, HashOutput};
use crate::schnorr::{PublicKey, Schnorr};
use crate::util::bitxor;

type Opening<B: EcBackend> = B::G1;
type Commitment<B: EcBackend> = B::G1;

const BYTE_DIFFICULTY: usize = 2;
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
    fn prelude(&self) {
        // FIXME compute this properly
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
}

impl<B: EcBackend, const M: usize> BaseProof<B, M> {
    fn check_pow(&self) -> bool {
        let state = [0u64; 8];

        true
    }

    fn verify(
        &self,
        fisch_iter: usize,
        prelude: (), // FIXME
        pk: &PublicKey<B>,
        com: &Commitment<B>,
        data_len: usize,
        kzg_settings: &B::KZGSettings,
    ) -> Result<bool, String> {
        let fft_settings = kzg_settings.get_fft_settings();

        check!(self.schnorr.verify(&pk));

        // Compute the indices
        let indices = derive_indices(fisch_iter, &self.schnorr.c, 8);
        let indices: [u64; 8] = indices.try_into().expect("FIXME support m != 8");

        let mut hash = HashOutput::default();
        // Verify openings
        for (idx, value, opening) in izip!(indices, self.data, &self.openings) {
            let value = B::Fr::from_u64(value);
            let x = get_point::<B>(&fft_settings, data_len, idx as usize);

            check!(kzg_settings.check_proof_single(&com, &opening, x, &value)?);

            let partial_pow = mine((), &self.schnorr.c, &self.schnorr.z, (), (), opening);
            hash = bitxor(hash, partial_pow);
        }

        check!(pow_pass(&hash, BYTE_DIFFICULTY));

        Ok(true)
    }
}

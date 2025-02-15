use hashing::derive_indices;
use itertools::izip;
use kzg_traits::{EcBackend, Fr, KZGSettings};

mod hashing;
mod schnorr;

type Opening<B: EcBackend> = B::G1;
type Commitment<B: EcBackend> = B::G1;
use schnorr::{PublicKey, Schnorr};

struct BaseProof<B: EcBackend, const M: usize> {
    schnorr: Schnorr<B>, // (a, c, z)
    data: [u64; M],
    openings: [Opening<B>; M],
}

struct Proof<B: EcBackend, const M: usize> {
    base_proofs: Vec<BaseProof<B, M>>,
}

impl<B: EcBackend, const M: usize> BaseProof<B, M> {}

impl<B: EcBackend, const M: usize> BaseProof<B, M> {
    fn verify(
        &self,
        pk: PublicKey<B>,
        com: Commitment<B>,
        kzg_settings: &B::KZGSettings,
    ) -> Result<bool, String> {
        if !self.schnorr.verify(&pk) {
            return Ok(false);
        }
        // Check the PoW
        // Compute the indices
        let fisch_iter = 1; // FIXME support multiple Fischlin iterations
        let indices = derive_indices(fisch_iter, &self.schnorr.c, 8);
        let indices: [u64; 8] = indices.try_into().expect("FIXME support m != 8");

        // Verify openings
        for (idx, value, opening) in izip!(indices, self.data, &self.openings) {
            let value = B::Fr::from_u64(value);
            let x = B::Fr::from_u64(idx); // FIXME properly compute x from idx
            if !kzg_settings.check_proof_single(&com, &opening, &x, &value)? {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

const T: usize = 10;
fn prove() {
    for c in 0..T {}
}

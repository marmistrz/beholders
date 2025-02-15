use kzg_traits::{EcBackend, KZGSettings, Fr};

mod schnorr;

type Opening<B: EcBackend> = B::G1;
type Commitment<B: EcBackend> = B::G1;
use schnorr::PublicKey;
struct BaseProof<B: EcBackend, const M: usize> {
    schnorr: (), // (c, z)
    data: [u64; M],
    openings: [Opening<B>; M],
}

impl<B: EcBackend, const M: usize> BaseProof<B, M> {
    fn verify(&self, pk: PublicKey<B>, com: Commitment<B>, kzg_settings: &B::KZGSettings) -> Result<bool, String> {
        // Verify schnorr
        // Verify openings
        for (value, opening) in self.data.iter().zip(self.openings.iter()) {
            let value = B::Fr::from_u64(*value);
            let x = B::Fr::default(); // FIXME derive indices from c
            if !kzg_settings.check_proof_single(&com, &opening, &x, &value)? {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

const T: usize = 10;
fn prove() {
    for c in 0..T {

    }
}
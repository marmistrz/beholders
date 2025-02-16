use itertools::izip;
use kzg_traits::{EcBackend, Fr, G1Mul, KZGSettings, G1};

use crate::check;
use crate::commitment::get_point;
use crate::hashing::{derive_indices, mine, pow_pass, HashOutput};
use crate::schnorr::{PublicKey, Schnorr, SecretKey};
use crate::util::bitxor;

type Opening<B: EcBackend> = B::G1;
type Commitment<B: EcBackend> = B::G1;
type Prelude = [u8; 64]; // FIXME type

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
    fn prelude(&self) -> [u8; 64] {
        use sha2::Digest;
        // FIXME: we should hash more than just the a_i's
        let a_i: Vec<u8> = self
            .base_proofs
            .iter()
            .map(|x| x.schnorr.a.to_bytes())
            .flatten()
            .collect();
        sha2::Sha512::digest(&a_i).into()
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

    fn prove(sk: SecretKey<B>, data: &[u64]) {
        let generator = B::G1::generator();
        // Compute the openings
        let mut openings: Vec<Opening<B>> = Vec::new();

        // Compute the Schnorr commitment
        let r_i: Vec<_> = (0..NFISCH).map(|_| B::Fr::rand()).collect();
        let a_i: Vec<_> = r_i.iter().map(|r| generator.mul(r)).collect();

        // TODO: prelude
        let prelude = [0u8; 64];

        for fisch_iter in 0..NFISCH {
            for c in 0..MAXC {
                // TODO check if direct add is faster
                let c = B::Fr::from_u64(c);
                let schnorr = Schnorr::<B>::prove(&sk, &r_i[fisch_iter], c.clone());

                let indices = derive_indices(fisch_iter, &c, 8);
                let indices: [u64; 8] = indices.try_into().expect("FIXME support m != 8");

                let mut hash = HashOutput::default();
                for idx in indices {
                    let partial_pow = mine(&prelude, &schnorr, (), (), &openings[0]);
                    hash = bitxor(hash, partial_pow);
                }
            }
        }
    }
}

impl<B: EcBackend, const NFISCH: usize> BaseProof<B, NFISCH> {
    fn check_pow(&self) -> bool {
        let state = [0u64; 8];

        true
    }

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

        check!(self.schnorr.verify(&pk));

        // Compute the indices
        let indices = derive_indices(fisch_iter, &self.schnorr.c, 8);
        let indices: [u64; 8] = indices.try_into().expect("FIXME support m != 8");

        let mut hash = HashOutput::default();

        // Verify openings and accumulate PoW
        for (idx, value, opening) in izip!(indices, self.data, &self.openings) {
            let value = B::Fr::from_u64(value);
            let x = get_point(fft_settings, data_len, idx as usize);

            check!(kzg_settings.check_proof_single(&com, &opening, x, &value)?);

            let partial_pow = mine(&prelude, &self.schnorr, (), (), opening);
            hash = bitxor(hash, partial_pow);
        }

        // Check PoW
        check!(pow_pass(&hash, BYTE_DIFFICULTY));

        Ok(true)
    }
}

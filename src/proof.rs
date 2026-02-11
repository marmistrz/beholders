use itertools::izip;
use kzg_traits::{Fr, G1Mul, KZGSettings, G1};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
use serde::{Deserialize, Serialize};

use crate::check;
use crate::commitment::{get_point, open_all_fk20, Commitment, Opening};
use crate::hashing::{derive_indices, individual_hash, pow_pass, prelude, HashOutput, Prelude};
use crate::schnorr::{maxc, PublicKey, Schnorr, SecretKey};
use crate::types::{TFr, TKZGSettings, TG1};
use crate::util::bitxor;

// TODO include beacon
/// A *successful* Fischlin iteration that managed to pass the proof-of-work.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct BaseProof {
    schnorr: Schnorr, // (a, c, z)
    data: Vec<TFr>,
    openings: Vec<Opening>,
}

/// Size of the data chunk in bytes
pub const CHUNK_SIZE: usize = 32;

/// A single Fischlin iteration.
/// If the solution was found, it contains a valid BaseProof.
/// Otherwise, it contains only the Schnorr commitment,
/// which is needed to compute the prelude.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum FischIter {
    Commitment(Commitment),
    BaseProof(BaseProof),
}

impl FischIter {
    /// Returns the Schnorr commitment `a`.
    fn a(&self) -> TG1 {
        match self {
            FischIter::Commitment(com) => *com,
            FischIter::BaseProof(bp) => bp.schnorr.a,
        }
    }

    /// Verifies a single Fischlin iteration.
    /// If only a commitment is present, the proof is considered invalid.
    pub fn verify(
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
        match self {
            FischIter::Commitment(_) => Ok(false),
            FischIter::BaseProof(bp) => bp.verify(
                fisch_iter,
                prelude,
                pk,
                com,
                data_len,
                kzg_settings,
                difficulty,
                mvalue,
            ),
        }
    }

    /// Attempts to solve a single Fischlin iteration.
    /// Returns either a valid BaseProof or just the Schnorr commitment if no solution was found.
    pub fn prove(
        fisch_iter: usize,
        prelude: Prelude,
        openings: &[Opening],
        r: &TFr,
        sk: &SecretKey,
        data: &[TFr],
        difficulty: u32,
        mvalue: usize,
    ) -> Self {
        match BaseProof::prove(
            fisch_iter, prelude, openings, r, sk, data, difficulty, mvalue,
        ) {
            Some(bp) => FischIter::BaseProof(bp),
            None => Self::Commitment(TG1::generator().mul(r)),
        }
    }
}

/// A complete beholder signature
/// `fisch_iters` contains some *invalid* proofs.
/// The signature is valid if at least half of them are valid.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Proof {
    pub fisch_iters: Vec<FischIter>,
}

impl Proof {
    fn prelude(&self, pk: &PublicKey, com: &Commitment) -> Prelude {
        let a_i = self.fisch_iters.iter().map(|x| x.a());
        prelude(pk, com, a_i)
    }

    /// Verifies the Beholder Signature.
    /// The signature is valid if at least half of the included Fischlin proofs are valid.
    ///
    /// # Arguments
    ///
    /// * `pk` - Schnorr public key.
    /// * `com` - KZG commitment for the data.
    /// * `data_len` - Length of the underlying data.
    /// * `kzg_settings` - KZG trusted setup.
    /// * `difficulty` - The bit difficulty, i.e., the required number of leading zeros.
    /// * `mvalue` - The number of indices to derive for each Schnorr transcript.
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
        assert_eq!(self.fisch_iters.len() % 2, 0);
        let prelude = self.prelude(pk, com);
        let verifications: Vec<_> = self
            .fisch_iters
            .par_iter()
            .enumerate()
            .map(|(fisch_iter, base_proof)| {
                base_proof.verify(
                    fisch_iter,
                    prelude,
                    pk,
                    com,
                    data_len,
                    kzg_settings,
                    difficulty,
                    mvalue,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let passed = verifications.iter().filter(|&&x| x).count();
        // for base_proof in &self.base_proofs {
        //     if !base_proof.verify(prelude, pk, com, data_len, kzg_settings, difficulty, mvalue)? {
        //         println!("Failed at base proof {}", base_proof.fisch_iter);
        //         return Ok(false);
        //     }
        // }
        println!(
            "Passed {}/{} Fischlin iterations",
            passed,
            self.fisch_iters.len(),
        );

        Ok(passed * 2 >= self.fisch_iters.len())
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
    /// Returns `Ok(sig, com)` where `sig` is the generated Beholder signature and `com` is the KZG commitment to the data.
    /// If the solution was not found, `sig` is `None`, which indicates that one should restart the signing with fresh randomness,
    /// otherwise sig is `Some`.
    /// or an `Err` with a string message in case of an error.
    pub fn prove(
        kzg_settings: &TKZGSettings,
        sk: SecretKey,
        data: &[u8],
        nfisch: usize,
        difficulty: u32,
        mvalue: usize,
    ) -> Result<(Option<Self>, Commitment), String> {
        assert!(
            data.len().is_power_of_two(),
            "Data length must be a power of two"
        );

        let data: Vec<_> = data
            .chunks_exact(CHUNK_SIZE)
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

        let fisch_iters: Vec<_> = (0..nfisch)
            .into_par_iter()
            .map(|fisch_iter| {
                FischIter::prove(
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
        assert_eq!(fisch_iters.len(), nfisch);

        let sig = Some(Self { fisch_iters });
        Ok((sig, com))
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

        check!(self.schnorr.c <= maxc(difficulty), "c too large");
        check!(self.schnorr.verify(pk), "Schnorr verification failed");

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

            check!(
                kzg_settings.check_proof_single(com, opening, x, value)?,
                "KZG verification failed"
            );

            let partial_pow =
                individual_hash(prelude, &self.schnorr, fisch_iter, k, *value, opening);
            hash = bitxor(hash, partial_pow);
        }

        // Check PoW
        check!(pow_pass(&hash, difficulty), "PoW verification failed");

        Ok(true)
    }

    /// Attempts to solve a single Fischlin iteration.
    /// Returns `Some(BaseProof)` if the proof-of-work successful, `None` if no solution was found.
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
        let maxc = maxc(difficulty); // Try up to 2^difficulty values of c
        for c in 0..maxc {
            let schnorr = Schnorr::prove(sk, r, c);

            let indices = derive_indices(fisch_iter, c, mvalue, data.len());
            assert_eq!(indices.len(), mvalue);
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
    use crate::schnorr::maxc;

    use super::*;
    const M: usize = 16;

    #[test]
    fn test_base_proof() {
        let data: Vec<TFr> = vec![4, 2137, 383, 4]
            .into_iter()
            .map(TFr::from_u64)
            .collect();

        let secrets_len = 16;
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

        let secrets_len = 16;
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
            .0
            .expect("No proof found");
        assert_eq!(proof.fisch_iters.len(), nfisch);
        for base_proof in &proof.fisch_iters {
            if let FischIter::BaseProof(base_proof) = base_proof {
                assert_eq!(base_proof.data.len(), M);
                assert!(base_proof.schnorr.verify(&pk));
            }
        }

        let data: Vec<_> = data
            .chunks_exact(CHUNK_SIZE)
            .map(|x| TFr::from_bytes_unchecked(x).unwrap())
            .collect();
        let poly = interpolate(kzg_settings.get_fft_settings(), &data);
        let com = kzg_settings.commit_to_poly(&poly).expect("commit");
        assert!(proof
            .verify(&pk, &com, data.len(), &kzg_settings, bit_difficulty, mvalue)
            .expect("KZG error"));
    }

    #[test]
    fn test_serialization() {
        let base_proof = BaseProof {
            schnorr: Schnorr {
                a: TG1::generator(),
                c: 42,
                z: TFr::from_u64(1337),
            },
            data: vec![TFr::from_u64(4); 16],
            openings: vec![Opening::default(); 16],
        };

        let serialized = bincode::serde::encode_to_vec(&base_proof, bincode::config::standard())
            .expect("Serialization failed");
        let (deserialized, _len): (BaseProof, _) =
            bincode::serde::decode_from_slice(&serialized, bincode::config::standard())
                .expect("Deserialization failed");
        assert_eq!(base_proof, deserialized);
    }

    /// Tests whether the proof verification only requires half of the Fischlin iterations to be valid.
    #[test]
    fn test_base_proof_tolerance() {
        let data = [4; 128];
        let bit_difficulty = 1;

        let secrets_len = 16;
        let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0; 32]);
        let fs = FsFFTSettings::new(4).unwrap();
        let kzg_settings: FsKZGSettings = FsKZGSettings::new(&s1, &s2, &s3, &fs, 7).unwrap();

        let g = TG1::generator();
        let sk = SecretKey::from_u64(2137);
        let pk = g.mul(&sk);

        let nfisch = 32;
        let mvalue: usize = 4;
        let (proof, com) = Proof::prove(&kzg_settings, sk, &data, nfisch, bit_difficulty, mvalue)
            .expect("KZG error");

        let mut proof = proof.expect("No solution found");
        assert_eq!(proof.fisch_iters.len(), nfisch);

        // Should verify
        let data: Vec<_> = data
            .chunks_exact(CHUNK_SIZE)
            .map(|x| TFr::from_bytes_unchecked(x).unwrap())
            .collect();

        assert!(proof
            .verify(&pk, &com, data.len(), &kzg_settings, bit_difficulty, mvalue)
            .expect("KZG error"));

        // Corrupt half of the valid proofs to simulate iterations that did not find a solution
        let mut nfailed = proof
            .fisch_iters
            .iter()
            .filter(|x| matches!(x, FischIter::Commitment(_)))
            .count();
        let half = proof.fisch_iters.len() / 2;
        for i in 0..proof.fisch_iters.len() {
            if nfailed >= half {
                break;
            }
            if let FischIter::BaseProof(_) = proof.fisch_iters[i] {
                println!("Corrupting proof {}", i);
                let a = proof.fisch_iters[i].a();
                proof.fisch_iters[i] = FischIter::Commitment(a);
                nfailed += 1;
            }
        }

        // Should still verify, since half are valid
        assert!(proof
            .verify(&pk, &com, data.len(), &kzg_settings, bit_difficulty, mvalue)
            .expect("KZG error"));

        // Corrupt one more proof, so less than half are valid
        for i in 0..proof.fisch_iters.len() {
            if let FischIter::BaseProof(_) = proof.fisch_iters[i] {
                println!("Corrupting proof {}", i);
                let a = proof.fisch_iters[i].a();
                proof.fisch_iters[i] = FischIter::Commitment(a);
                break;
            }
        }

        // Should now fail verification
        assert!(!proof
            .verify(&pk, &com, data.len(), &kzg_settings, bit_difficulty, mvalue)
            .expect("KZG error"));
    }

    #[test]
    fn base_proof_fails_if_c_exceeds_max() {
        let data: Vec<TFr> = vec![1, 2, 3, 4].into_iter().map(TFr::from_u64).collect();

        let secrets_len = 16;
        let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0; 32]);
        let fs = FsFFTSettings::new(4).unwrap();
        let kzg_settings: FsKZGSettings = FsKZGSettings::new(&s1, &s2, &s3, &fs, 7).unwrap();

        let (_com, openings) = open_all_fk20(&kzg_settings, &data).expect("openings");
        assert_eq!(openings.len(), data.len());

        let g = TG1::generator();
        let r = FsFr::from_u64(42);
        let sk = SecretKey::from_u64(1234);
        let pk = g.mul(&sk);
        let byte_difficulty = 2;
        let mvalue: usize = 4;
        let fisch_iter = 0;
        let prelude = [0; 8];

        // Try to create a Schnorr proof with c > maxc(difficulty)
        let invalid_c = maxc(byte_difficulty) + 1;
        let invalid_c_max = invalid_c + 2000;
        let mut base_proof = None;
        for c in invalid_c..invalid_c_max {
            let schnorr = Schnorr::prove(&sk, &r, c);

            let indices = derive_indices(fisch_iter, c, mvalue, data.len());
            let indices: Vec<usize> = indices.into_iter().collect();
            let data_selected: Vec<_> = indices.iter().map(|&i| data[i]).collect();
            let openings_selected: Vec<_> = indices.iter().map(|&i| openings[i]).collect();

            // Manually compute the PoW hash
            let mut hash = HashOutput::default();
            for (k, (val, opening)) in
                izip!(data_selected.iter(), openings_selected.iter()).enumerate()
            {
                let k = k.try_into().unwrap();
                let partial_pow = individual_hash(prelude, &schnorr, fisch_iter, k, *val, opening);
                hash = bitxor(hash, partial_pow);
            }
            // Only use the proof if PoW passes
            if pow_pass(&hash, byte_difficulty) {
                base_proof = Some(BaseProof {
                    schnorr,
                    data: data_selected,
                    openings: openings_selected,
                });
                break;
            }
        }
        let base_proof = base_proof.expect("No valid PoW found for invalid_c");

        let poly = interpolate(kzg_settings.get_fft_settings(), &data);
        let com = kzg_settings.commit_to_poly(&poly).expect("commit");

        let result = base_proof
            .verify(
                fisch_iter,
                prelude,
                &pk,
                &com,
                data.len(),
                &kzg_settings,
                byte_difficulty,
                mvalue,
            )
            .expect("KZG error");

        assert!(!result, "Proof should fail if c exceeds maxc");
    }
}

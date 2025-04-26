use std::time::Instant;

use kzg_traits::{eth, FFTFr, FFTSettings, FK20SingleSettings, Fr, KZGSettings, Poly};
use serde::{Deserialize, Serialize};

use crate::types::{TFFTSettings, TFK20SingleSettings, TFr, TKZGSettings, TPoly, TG1, TG2};

/// KZG opening
pub type Opening = TG1;
/// Polynomial Commitment (KZG) value
pub type Commitment = TG1;

#[derive(Debug, Serialize, Deserialize)]
pub struct TrustedSetup {
    pub g1_monomial: Vec<TG1>,
    pub g1_lagrange: Vec<TG1>,
    pub g2_monomial: Vec<TG2>,
}

impl TrustedSetup {
    pub fn from_kzg_settings(kzg_settings: TKZGSettings) -> Self {
        Self {
            g1_monomial: kzg_settings.g1_values_monomial,
            g1_lagrange: kzg_settings.g1_values_lagrange_brp,
            g2_monomial: kzg_settings.g2_values_monomial,
        }
    }

    pub fn into_kzg_settings(self, fs: &TFFTSettings) -> Result<TKZGSettings, String> {
        TKZGSettings::new(
            &self.g1_monomial,
            &self.g1_lagrange,
            &self.g2_monomial,
            fs,
            eth::FIELD_ELEMENTS_PER_CELL,
        )
    }
}

pub(crate) fn interpolate<TFr, TFFT, TPoly>(settings: &TFFT, data: &[TFr]) -> TPoly
where
    TFr: Fr,
    TFFT: FFTSettings<TFr> + FFTFr<TFr>,
    TPoly: Poly<TFr>,
{
    let coeffs = settings.fft_fr(data, true).unwrap();
    TPoly::from_coeffs(coeffs.as_slice())
}

pub(crate) fn get_point<TFr: Fr>(
    settings: &impl FFTSettings<TFr>,
    data_len: usize,
    i: usize,
) -> &TFr {
    let roots = settings.get_roots_of_unity();
    let stride = (roots.len() - 1) / data_len;
    &roots[i * stride]
}

pub fn open_all_fk20(
    kzg_settings: &TKZGSettings,
    data: &[TFr],
) -> Result<(Commitment, Vec<Opening>), String> {
    let start = Instant::now();

    let fft_settings = kzg_settings.get_fft_settings();
    let fk20_settings = TFK20SingleSettings::new(kzg_settings, 2 * data.len())?;
    let poly: TPoly = interpolate(fft_settings, data);
    let com = kzg_settings.commit_to_poly(&poly)?;
    let fk20 = fk20_settings.data_availability_optimized(&poly)?;
    let openings = fk20
        .into_iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 0)
        .map(|(_, x)| x)
        .collect();

    let duration = start.elapsed();
    println!("FK20 time: {:?}", duration);
    Ok((com, openings))
}

#[cfg(test)]
mod tests {
    use crate::types::TFFTSettings;

    use super::*;
    use kzg::{
        types::{fft_settings::FsFFTSettings, kzg_settings::FsKZGSettings, poly::FsPoly},
        utils::generate_trusted_setup,
    };
    use kzg_traits::{FFTSettings, Fr, KZGSettings, Poly};

    fn open_all(kzg_settings: &TKZGSettings, data: &[TFr]) -> Result<Vec<Opening>, String> {
        let fft_settings = kzg_settings.get_fft_settings();
        let poly: TPoly = interpolate(fft_settings, data);
        data.iter()
            .enumerate()
            .map(|(i, _)| {
                let x = get_point(fft_settings, data.len(), i);
                kzg_settings.compute_proof_single(&poly, x)
            })
            .collect()
    }

    #[test]
    fn test_interpolate() {
        let data: Vec<TFr> = vec![4, 2137, 383, 4]
            .into_iter()
            .map(TFr::from_u64)
            .collect();
        let fft_settings = TFFTSettings::new(15).unwrap();
        let poly: FsPoly = interpolate(&fft_settings, &data);

        for (i, orig) in data.iter().enumerate() {
            let root = get_point(&fft_settings, data.len(), i);
            let val = poly.eval(root);
            assert_eq!(val, *orig, "i={}", i);
        }
    }

    #[test]
    fn test_interpolate_long() {
        let data: Vec<TFr> = vec![8; 128].into_iter().map(TFr::from_u64).collect();
        let fft_settings = TFFTSettings::new(15).unwrap();
        let poly: FsPoly = interpolate(&fft_settings, &data);

        for (i, orig) in data.iter().enumerate() {
            let root = get_point(&fft_settings, data.len(), i);
            let val = poly.eval(root);
            assert_eq!(val, *orig, "i={}", i);
        }
    }

    #[test]
    fn test_large_trusted_setup() {
        let secrets_len = 1024;
        let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0; 32]);
        let fs = FsFFTSettings::new(10).unwrap();

        let _kzg_settings: FsKZGSettings = FsKZGSettings::new(&s1, &s2, &s3, &fs, 64).unwrap();
    }

    #[test]
    fn test_prove() {
        let data: Vec<TFr> = vec![4, 2137, 383, 4]
            .into_iter()
            .map(TFr::from_u64)
            .collect();

        let secrets_len = 16;
        let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0; 32]);
        let fs = FsFFTSettings::new(4).unwrap();
        let kzg_settings: FsKZGSettings = FsKZGSettings::new(&s1, &s2, &s3, &fs, 7).unwrap();

        let fft_settings = kzg_settings.get_fft_settings();

        let poly = interpolate(fft_settings, &data);
        let com = kzg_settings.commit_to_poly(&poly).expect("commit");

        for (i, val) in data.iter().enumerate() {
            let x = get_point(fft_settings, data.len(), i);

            assert_eq!(poly.eval(x), *val, "value");
            let proof = kzg_settings.compute_proof_single(&poly, x).expect("prove");
            let res = kzg_settings
                .check_proof_single(&com, &proof, x, val)
                .expect("verify");
            assert!(res, "Proof did not verify for i={i}");
        }
    }

    #[test]
    fn test_open_all() {
        let data: Vec<TFr> = vec![4, 2137, 383, 4]
            .into_iter()
            .map(TFr::from_u64)
            .collect();

        let secrets_len = 16;
        let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0; 32]);
        let fs = FsFFTSettings::new(4).unwrap();
        let kzg_settings: FsKZGSettings = FsKZGSettings::new(&s1, &s2, &s3, &fs, 7).unwrap();

        let fft_settings = kzg_settings.get_fft_settings();

        let openings = open_all(&kzg_settings, &data).expect("openings");
        let poly = interpolate(fft_settings, &data);
        let com = kzg_settings.commit_to_poly(&poly).expect("commit");

        for ((i, val), proof) in data.iter().enumerate().zip(openings.iter()) {
            let x = get_point(fft_settings, data.len(), i);

            assert_eq!(poly.eval(x), *val, "value");
            let res = kzg_settings
                .check_proof_single(&com, proof, x, val)
                .expect("verify");
            assert!(res, "Proof did not verify for i = {}", i);
        }
    }

    #[test]
    fn test_prove_fk20() {
        let poly_len: usize = 4;

        const TRUSTED_SETUP_FILE: &str = "trusted_setup.txt";
        let ks = kzg::eip_4844::load_trusted_setup_filename_rust(TRUSTED_SETUP_FILE)
            .expect("loading trusted setup");
        let fs = ks.get_fft_settings();
        let fk = TFK20SingleSettings::new(&ks, 2 * poly_len).unwrap();

        // Commit to the polynomial
        let data: Vec<TFr> = vec![4, 2137, 383, 4]
            .into_iter()
            .map(TFr::from_u64)
            .collect();
        let p: FsPoly = interpolate(fs, &data);
        assert_eq!(p.coeffs.len(), poly_len);

        let commitment = ks.commit_to_poly(&p).unwrap();

        // Generate the proofs
        let all_proofs = fk.data_availability_optimized(&p).unwrap();
        let direct = open_all(&ks, &data).unwrap();
        let (idx, _) = all_proofs
            .iter()
            .enumerate()
            .find(|&(_, &x)| x == direct[1])
            .expect("find");
        assert_eq!(idx, 2);

        // Verify the proof at each root of unity
        for (i, proof) in all_proofs.iter().enumerate().take(2 * poly_len) {
            if i % 2 == 1 {
                continue;
            }
            let i = i / 2;
            let x = get_point(fs, data.len(), i); //fs.get_roots_of_unity_at(i);
            let y = p.eval(x);
            assert!(
                ks.check_proof_single(&commitment, proof, x, &y).unwrap(),
                "{i}"
            );
        }
    }

    #[test]
    fn test_open_all_matches() {
        // let n: usize = 5;
        // let n_len: usize = 1 << n;
        // let secrets_len = 2 * n_len;

        // // assert!(n_len >= 2 * poly_len);

        // // FIXME: this also fails with the trusted setup
        // // Initialise the secrets and data structures
        // let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0; 32]);
        // let fs = FsFFTSettings::new(n).unwrap();
        // let ks = FsKZGSettings::new(&s1, &s2, &s3, &fs, 4).unwrap();

        const TRUSTED_SETUP_FILE: &str = "trusted_setup.txt";
        let ks = kzg::eip_4844::load_trusted_setup_filename_rust(TRUSTED_SETUP_FILE)
            .expect("loading trusted setup");

        // Commit to the polynomial
        let data: Vec<TFr> = vec![4, 2137, 383, 4]
            .into_iter()
            .map(TFr::from_u64)
            .collect();

        let (_com, all_proofs) = open_all_fk20(&ks, &data).unwrap();
        let direct = open_all(&ks, &data).unwrap();
        assert_eq!(all_proofs, direct);
    }

    #[test]
    fn test_prove_trusted_setup() {
        let data: Vec<TFr> = vec![4, 2137, 383, 4]
            .into_iter()
            .map(TFr::from_u64)
            .collect();

        const TRUSTED_SETUP_FILE: &str = "trusted_setup.txt";
        let kzg_settings = kzg::eip_4844::load_trusted_setup_filename_rust(TRUSTED_SETUP_FILE)
            .expect("loading trusted setup");
        let fft_settings = kzg_settings.get_fft_settings();

        let poly = interpolate(fft_settings, &data);
        let com = kzg_settings.commit_to_poly(&poly).expect("commit");

        for (i, val) in data.iter().enumerate() {
            let x = get_point(fft_settings, data.len(), i);

            assert_eq!(poly.eval(x), *val, "value");
            let proof = kzg_settings.compute_proof_single(&poly, x).expect("prove");
            let res = kzg_settings
                .check_proof_single(&com, &proof, x, val)
                .expect("verify");
            assert!(res, "Proof did not verify for i = {}", i);
        }
    }
}

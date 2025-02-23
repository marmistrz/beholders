use kzg_traits::{FFTFr, FFTSettings, FK20SingleSettings, Fr, KZGSettings, Poly};

use crate::types::{TFK20SingleSettings, TKZGSettings, TPoly, TG1};

/// KZG opening
pub(crate) type Opening = TG1;
/// Polynomial Commitment (KZG) value
pub(crate) type Commitment = TG1;

pub(crate) fn interpolate<TFr, TFFT, TPoly>(settings: &TFFT, data: &[u64]) -> TPoly
where
    TFr: Fr,
    TFFT: FFTSettings<TFr> + FFTFr<TFr>,
    TPoly: Poly<TFr>,
{
    let data = data.iter().map(|x| TFr::from_u64(*x)).collect::<Vec<_>>();
    let coeffs = settings.fft_fr(data.as_slice(), true).unwrap();
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

// TODO migrate to this function
pub fn open_all_fk20(kzg_settings: &TKZGSettings, data: &[u64]) -> Result<Vec<Opening>, String> {
    let fft_settings = kzg_settings.get_fft_settings();
    let fk20_settings = TFK20SingleSettings::new(kzg_settings, 2 * data.len())?;
    let poly: TPoly = interpolate(fft_settings, data);
    let fk20 = fk20_settings.data_availability_optimized(&poly)?;
    Ok(fk20
        .into_iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 0)
        .map(|(_, x)| x)
        .collect())
}

#[cfg(test)]
mod tests {
    use crate::types::TFFTSettings;

    use super::*;
    use kzg::{
        types::{fft_settings::FsFFTSettings, fr::FsFr, kzg_settings::FsKZGSettings, poly::FsPoly},
        utils::generate_trusted_setup,
    };
    use kzg_traits::{FFTSettings, Fr, KZGSettings, Poly};

    fn open_all(kzg_settings: &TKZGSettings, data: &[u64]) -> Result<Vec<Opening>, String> {
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
        let data = [4, 2137, 383, 4]; //, 5, 1, 5, 7];
        let fft_settings = TFFTSettings::new(15).unwrap();
        let poly: FsPoly = interpolate(&fft_settings, &data);

        for (i, orig) in data.iter().enumerate() {
            let root = get_point(&fft_settings, data.len(), i);
            let val = poly.eval(root);
            assert_eq!(
                val,
                FsFr::from_u64(*orig),
                "root={:?} orig={} i={}",
                root,
                orig,
                i
            );
        }
    }

    #[test]
    fn test_interpolate_long() {
        let data = [0; 64]; //, 5, 1, 5, 7];
        let fft_settings = TFFTSettings::new(15).unwrap();
        let poly: FsPoly = interpolate(&fft_settings, &data);

        for (i, orig) in data.iter().enumerate() {
            let root = get_point(&fft_settings, data.len(), i);
            let val = poly.eval(root);
            assert_eq!(
                val,
                FsFr::from_u64(*orig),
                "root={:?} orig={} i={}",
                root,
                orig,
                i
            );
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
        let data = [4, 2137, 383, 4]; //, 5, 1, 5, 7];

        let secrets_len = 15;
        let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0; 32]);
        let fs = FsFFTSettings::new(4).unwrap();
        let kzg_settings: FsKZGSettings = FsKZGSettings::new(&s1, &s2, &s3, &fs, 7).unwrap();

        let fft_settings = kzg_settings.get_fft_settings();

        let poly = interpolate(fft_settings, &data);
        let com = kzg_settings.commit_to_poly(&poly).expect("commit");

        for (i, val) in data.iter().enumerate() {
            let value = FsFr::from_u64(*val);
            let x = get_point(fft_settings, data.len(), i);

            assert_eq!(poly.eval(x), value, "value");
            let proof = kzg_settings.compute_proof_single(&poly, x).expect("prove");
            let res = kzg_settings
                .check_proof_single(&com, &proof, x, &value)
                .expect("verify");
            assert!(res, "Proof did not verify for i = {}, value = {}", i, val);
        }
    }

    #[test]
    fn test_open_all() {
        let data: [u64; 4] = [4, 2137, 383, 4]; //, 5, 1, 5, 7];

        let secrets_len = 15;
        let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0; 32]);
        let fs = FsFFTSettings::new(4).unwrap();
        let kzg_settings: FsKZGSettings = FsKZGSettings::new(&s1, &s2, &s3, &fs, 7).unwrap();

        let fft_settings = kzg_settings.get_fft_settings();

        let openings = open_all(&kzg_settings, &data).expect("openings");
        let poly = interpolate(fft_settings, &data);
        let com = kzg_settings.commit_to_poly(&poly).expect("commit");

        for ((i, val), proof) in data.iter().enumerate().zip(openings.iter()) {
            let value = FsFr::from_u64(*val);
            let x = get_point(fft_settings, data.len(), i);

            assert_eq!(poly.eval(x), value, "value");
            let res = kzg_settings
                .check_proof_single(&com, proof, x, &value)
                .expect("verify");
            assert!(res, "Proof did not verify for i = {}, value = {}", i, val);
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
        let data: [u64; 4] = [4, 2137, 383, 4]; //, 5, 1, 5, 7];
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
        let n: usize = 5;
        let n_len: usize = 1 << n;
        let secrets_len = n_len + 1;

        // assert!(n_len >= 2 * poly_len);

        // FIXME: this also fails with the trusted setup
        // Initialise the secrets and data structures
        let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0; 32]);
        let fs = FsFFTSettings::new(n).unwrap();
        let ks = FsKZGSettings::new(&s1, &s2, &s3, &fs, 4).unwrap();

        // const TRUSTED_SETUP_FILE: &str = "trusted_setup.txt";
        // let ks = kzg::eip_4844::load_trusted_setup_filename_rust(TRUSTED_SETUP_FILE)
        //     .expect("loading trusted setup");

        // Commit to the polynomial
        let data: [u64; 4] = [4, 2137, 383, 4]; //, 5, 1, 5, 7];

        let all_proofs = open_all_fk20(&ks, &data).unwrap();
        let direct = open_all(&ks, &data).unwrap();
        assert_eq!(all_proofs, direct);
    }

    #[test]
    fn test_prove_trusted_setup() {
        let data = [4, 2137, 383, 4]; //, 5, 1, 5, 7];
                                      // let mut data = ;

        const TRUSTED_SETUP_FILE: &str = "trusted_setup.txt";
        let kzg_settings = kzg::eip_4844::load_trusted_setup_filename_rust(TRUSTED_SETUP_FILE)
            .expect("loading trusted setup");
        let fft_settings = kzg_settings.get_fft_settings();

        let poly = interpolate(fft_settings, &data);
        let com = kzg_settings.commit_to_poly(&poly).expect("commit");

        for (i, val) in data.iter().enumerate() {
            let value = FsFr::from_u64(*val);
            let x = get_point(fft_settings, data.len(), i);

            assert_eq!(poly.eval(x), value, "value");
            let proof = kzg_settings.compute_proof_single(&poly, x).expect("prove");
            let res = kzg_settings
                .check_proof_single(&com, &proof, x, &value)
                .expect("verify");
            assert!(res, "Proof did not verify for i = {}, value = {}", i, val);
        }
    }
}

use kzg_traits::{EcBackend, FFTFr, FFTSettings, Fr, KZGSettings, Poly};

pub(crate) type Opening<B> = <B as EcBackend>::G1;

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

// TODO use fk20
pub fn open_all<B: EcBackend>(
    kzg_settings: &B::KZGSettings,
    data: &[u64],
) -> Result<Vec<Opening<B>>, String> {
    let fft_settings = kzg_settings.get_fft_settings();
    let poly: B::Poly = interpolate(fft_settings, data);
    data.iter()
        .enumerate()
        .map(|(i, _)| {
            let x = get_point(fft_settings, data.len(), i);
            kzg_settings.compute_proof_single(&poly, x)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kzg::{
        eip_7594::BlstBackend,
        types::{fft_settings::FsFFTSettings, fr::FsFr, kzg_settings::FsKZGSettings, poly::FsPoly},
        utils::generate_trusted_setup,
    };
    use kzg_traits::{EcBackend, FFTSettings, Fr, KZGSettings, Poly};
    type Backend = BlstBackend;

    #[test]
    fn test_interpolate() {
        let data = [4, 2137, 383, 4]; //, 5, 1, 5, 7];
        let fft_settings = <Backend as EcBackend>::FFTSettings::new(15).unwrap();
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
        let fft_settings = <Backend as EcBackend>::FFTSettings::new(15).unwrap();
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
    fn test_prove() {
        let data = [4, 2137, 383, 4]; //, 5, 1, 5, 7];

        let secrets_len = 15;
        let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0; 32]);
        let fs = FsFFTSettings::new(4).unwrap();
        let kzg_settings: FsKZGSettings = FsKZGSettings::new(&s1, &s2, &s3, &fs, 7).unwrap();

        // FIXME: this should work with the Ethereum trusted setup
        // let kzg_settings =
        //     load_trusted_setup_filename_rust(TRUSTED_SETUP_FILE).expect("loading trusted setup");
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

    // #[test]
    // fn test_prove() {
    //     let data = [4, 2137, 383, 4]; //, 5, 1, 5, 7];
    //     let interpolator = Interpolator::<Backend>::new(15).unwrap();
    //     let kzg_settings =
    //         load_trusted_setup_filename_rust(TRUSTED_SETUP_FILE).expect("loading trusted setup");

    //     let secrets_len = 15;
    //     let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0;32]);
    //     let fs = &kzg_settings.fs;
    //     let kzg_settings: FsKZGSettings = FsKZGSettings::new(&s1, &s2, &s3, &fs, 7).unwrap();

    //     let poly = interpolate::<Backend>(&fs, &data);
    //     let com = kzg_settings.commit_to_poly(&poly).expect("commit");

    //     let roots = &interpolator.fft_settings.roots_of_unity;
    //     let stride = (roots.len() - 1)/data.len();

    //     for (i, val) in data.iter().enumerate() {
    //         let value = FsFr::from_u64(*val);
    //         let x = roots[i*stride];

    //         assert_eq!(poly.eval(&x), value, "value");
    //         let proof = kzg_settings.compute_proof_single(&poly, &x).expect("prove");
    //         let res = kzg_settings.check_proof_single(&com, &proof, &x, &value).expect("verify");
    //         assert!(res, "Proof did not verify for i = {}, value = {}", i, val);
    //         // assert_eq!(val, FsFr::from_u64(*orig), "root={:?} orig={} i={}", root, orig, i);
    //     }
    // }
}

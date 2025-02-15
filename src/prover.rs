use std::time::Instant;

use kzg::{eip_4844::load_trusted_setup_filename_rust, eip_7594::BlstBackend};
use kzg_traits::{EcBackend, FFTFr, FFTSettings, Fr, KZGSettings, Poly};

const TRUSTED_SETUP_FILE: &str = "trusted_setup.txt";

type Backend = BlstBackend;

pub(crate) struct Prover<B: EcBackend> {
    kzg_settings: B::KZGSettings,
    // interpolator: Interpolator<B>,
}

// struct Interpolator<B: EcBackend> {
//     fft_settings: B::FFTSettings,
// }

// impl<B: EcBackend> Interpolator<B> {
//     fn new(log_max_len: usize) -> Result<Self, String> {
//         let fft_settings = B::FFTSettings::new(log_max_len)?;
//         Ok(Self { fft_settings })
//     }

//     fn interpolate(&self, data: &[u64]) -> B::Poly {
//         let data = data.iter().map(|x| B::Fr::from_u64(*x)).collect::<Vec<_>>();
//         let coeffs = self.fft_settings.fft_fr(data.as_slice(), true).unwrap();
//         Poly::from_coeffs(coeffs.as_slice())
//     }
// }

impl<B: EcBackend> Prover<B> {
    fn new(kzg_settings: B::KZGSettings) -> Result<Self, String> {
        // let interpolator = Interpolator::new(log_max_len)?;
        Ok(Self { kzg_settings })
    }

    fn prove(&self, data: &[u64]) {
        let interdata = InterpolatedData::<B>::new(&self.kzg_settings.get_fft_settings(), data);
        // let poly = interpolate::<B>(self.kzg_settings.get_fft_settings(), data);
        // let commitment = self.kzg_settings.commit_to_poly(&poly);
        for (i, val) in data.iter().enumerate() {
            // let proof = self.kzg_settings.compute_proof_single(&poly, i as u64);
        }
        println!("Hello, world!");
    }
}

fn main() {
    let data = [1, 2, 3, 4];
    let start = Instant::now();
    let trusted_setup =
        load_trusted_setup_filename_rust(TRUSTED_SETUP_FILE).expect("loading trusted setup");
    let prover = Prover::<Backend>::new(trusted_setup).unwrap();
    let duration = start.elapsed();

    println!("Initialization time: {:?}", duration);
    prover.prove(&data);
}

fn interpolate<B: EcBackend>(settings: &B::FFTSettings, data: &[u64]) -> B::Poly {
    let data = data.iter().map(|x| B::Fr::from_u64(*x)).collect::<Vec<_>>();
    let coeffs = settings.fft_fr(data.as_slice(), true).unwrap();
    Poly::from_coeffs(coeffs.as_slice())
}

struct InterpolatedData<'a, B: EcBackend> {
    data: &'a [u64],
    poly: B::Poly,
    settings: &'a B::FFTSettings,
}

impl<'a, B: EcBackend> InterpolatedData<'a, B> {
    fn new(settings: &'a B::FFTSettings, data: &'a [u64]) -> Self {
        let poly = interpolate::<B>(settings, data);
        Self {
            data,
            poly,
            settings,
        }
    }

    fn stride(&self) -> usize {
        let roots = self.settings.get_roots_of_unity();
        (roots.len() - 1) / self.data.len()
    }

    fn get(&self, i: usize) {
        let roots = self.settings.get_roots_of_unity();
        let stride = self.stride();
        let root = roots.get(i * stride).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use kzg::types::{fft_settings::FsFFTSettings, fr::FsFr};

    use super::*;

    #[test]
    fn test_interpolate() {
        let data = [4, 2137, 383, 4]; //, 5, 1, 5, 7];
        let fft_settings = FsFFTSettings::new(15).unwrap();
        let poly = interpolate::<Backend>(&fft_settings, &data);

        let roots = &fft_settings.roots_of_unity;
        let stride = (roots.len() - 1) / data.len();

        for (i, orig) in data.iter().enumerate() {
            let root = roots[i * stride];
            let val = poly.eval(&root);
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

        // let secrets_len = 15;
        // let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0;32]);
        // let fs = FsFFTSettings::new(4).unwrap();
        // let kzg_settings: FsKZGSettings = FsKZGSettings::new(&s1, &s2, &s3, &fs, 7).unwrap();

        let kzg_settings =
            load_trusted_setup_filename_rust(TRUSTED_SETUP_FILE).expect("loading trusted setup");
        let fft_settings = kzg_settings.get_fft_settings();

        let poly = interpolate::<Backend>(&fft_settings, &data);
        let com = kzg_settings.commit_to_poly(&poly).expect("commit");

        let roots = &fft_settings.roots_of_unity;
        let stride = (roots.len() - 1) / data.len();

        for (i, val) in data.iter().enumerate() {
            let value = FsFr::from_u64(*val);
            let x = roots[i * stride];

            assert_eq!(poly.eval(&x), value, "value");
            let proof = kzg_settings.compute_proof_single(&poly, &x).expect("prove");
            let res = kzg_settings
                .check_proof_single(&com, &proof, &x, &value)
                .expect("verify");
            assert!(res, "Proof did not verify for i = {}, value = {}", i, val);
            // assert_eq!(val, FsFr::from_u64(*orig), "root={:?} orig={} i={}", root, orig, i);
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

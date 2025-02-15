use kzg_traits::{EcBackend, FFTFr, FFTSettings, Fr, KZGSettings, Poly};

pub(crate) fn interpolate<B: EcBackend>(settings: &B::FFTSettings, data: &[u64]) -> B::Poly {
    let data = data.iter().map(|x| B::Fr::from_u64(*x)).collect::<Vec<_>>();
    let coeffs = settings.fft_fr(data.as_slice(), true).unwrap();
    Poly::from_coeffs(coeffs.as_slice())
}

// pub(crate) fn stride<B: EcBackend>(settings: &B::FFTSettings, data: &[u64]) -> usize {
//     let roots = settings.get_roots_of_unity();
//     (roots.len() - 1) / data.len()
// }

pub(crate) fn get_point<B: EcBackend>(
    settings: &B::FFTSettings,
    data_len: usize,
    i: usize,
) -> &B::Fr {
    let roots = settings.get_roots_of_unity();
    let stride = (roots.len() - 1) / data_len;
    &roots[i * stride]
}

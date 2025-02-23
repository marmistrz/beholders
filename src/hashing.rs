use kzg_traits::{Fr, G1};
// use log::debug;
use sha2::{
    compress512,
    digest::{consts::U128, generic_array::GenericArray},
};

use crate::schnorr::Schnorr;

pub(crate) type HashOutput = [u64; 8];
pub(crate) type Prelude = HashOutput;

pub(crate) fn prelude<TG1: G1>(a_i: impl Iterator<Item = TG1>) -> Prelude {
    use sha2::Digest;
    // FIXME: we should hash more than just the a_i's
    let a_i: Vec<u8> = a_i.flat_map(|x| x.to_bytes()).collect();
    let hash: [u8; 64] = sha2::Sha512::digest(&a_i).into();
    bytemuck::cast(hash)
}

pub(crate) fn derive_indices(
    i: usize,
    c: &impl Fr,
    num_indices: usize,
    data_len: usize,
) -> Vec<usize> {
    let mut state = [0u64; 8];
    let mut input = [0u8; 128];
    input[0..8].clone_from_slice(&i.to_le_bytes());
    input[8..40].clone_from_slice(&c.to_bytes());

    let blocks: &GenericArray<_, U128> = GenericArray::from_slice(&input);
    compress512(&mut state, &[*blocks]);

    assert!(num_indices <= 8, "FIXME support m > 8");
    state
        .map(|x| {
            let x: usize = x.try_into().unwrap();
            x % data_len
        })
        .into_iter()
        .take(num_indices)
        .collect()
}

// prelude: 32 bytes
// c: 32 bytes
// z: 32 bytes
// k: 4/8 bytes
// val: 32 bytes
// opening: 48 bytes
// TOTAL:
pub(crate) fn individual_hash(
    prelude: Prelude,
    schnorr: &Schnorr,
    k: u8,
    val: u64,
    opening: &impl G1,
) -> HashOutput {
    let mut state: HashOutput = prelude;
    let mut input = [0u8; 128];

    let Schnorr { c, z, .. } = schnorr;

    // input[0..8].clone_from_slice(&.to_le_bytes());
    input[0..48].clone_from_slice(&opening.to_bytes());
    input[48..80].clone_from_slice(&c.to_bytes());
    input[80..112].clone_from_slice(&z.to_bytes());
    input[112..120].clone_from_slice(&val.to_le_bytes());
    input[120] = k;

    let blocks: &GenericArray<_, U128> = GenericArray::from_slice(&input); //[c.to_bytes(), pad].iter().flatten().into();
    compress512(&mut state, &[*blocks]);
    state
}

/// Returns true if `hash_output` has at least `difficulty` leading zeros (little-endian) / trailing zeros (big-endian).
pub(crate) fn pow_pass(hash_output: &HashOutput, difficulty: u32) -> bool {
    assert!(difficulty <= 64, "Only difficulty <= 64 is supported");
    hash_output[0].trailing_zeros() >= difficulty
}

#[cfg(test)]
mod tests {
    use kzg::types::{fr::FsFr, g1::FsG1};
    use test_case::test_case;

    use super::*;

    #[test_case(8; "m=8")]
    #[test_case(6; "m=6")]
    fn test_derive_indices(m: usize) {
        let i = 1;
        let c = FsFr::one();
        let data_len = 10;
        let indices = derive_indices(i, &c, m, data_len);
        assert_eq!(indices.len(), m);
        indices.iter().for_each(|&x| assert!(x < data_len));
    }

    // #[test]
    // fn test_derive_indices2() {
    //     let i = 1;
    //     let c = FsFr::one();
    //     let m = 10;
    //     let indices = derive_indices(i, &c, m);
    //     assert_eq!(indices.len(), m);
    // }

    #[test]
    fn test_pow_pass() {
        let mut hash_output = [0u64; 8];
        assert!(pow_pass(&hash_output, 1));
        assert!(pow_pass(&hash_output, 64));
        hash_output[0] = (u8::MAX as u64) + 1;
        assert!(pow_pass(&hash_output, 8));
        assert!(!pow_pass(&hash_output, 9));
    }

    #[test]
    fn test_invididual_hash() {
        let schnorr = Schnorr::prove(&Default::default(), &Default::default(), Default::default());
        let prelude = [0u64; 8];
        let opening = FsG1::generator();
        individual_hash(prelude, &schnorr, 0, 0, &opening);
    }
}

use kzg_traits::{Fr, G1};
use sha2::{
    compress512,
    digest::{consts::U128, generic_array::GenericArray},
};

pub(crate) type HashOutput = [u64; 8];

pub(crate) fn derive_indices(i: usize, c: &impl Fr, m: usize) -> Vec<u64> {
    let mut state = [0u64; 8];
    let mut input = [0u8; 128];
    input[0..8].clone_from_slice(&i.to_le_bytes());
    input[8..40].clone_from_slice(&c.to_bytes());

    let blocks: &GenericArray<_, U128> = GenericArray::from_slice(&input); //[c.to_bytes(), pad].iter().flatten().into();
    compress512(&mut state, &[*blocks]);

    assert_eq!(m, 8, "FIXME support m != 8");
    Vec::from(state)
}

// c: 16/32 bytes
// z: 32 bytes
// k: 4/8 bytes
// val: 32 bytes
// opening: 48 bytes
pub(crate) fn mine(
    prelude: &[u8; 64],
    c: &impl Fr,
    z: &impl Fr,
    k: (),
    val: (),
    opening: &impl G1,
) -> HashOutput {
    // TODO finish this
    let mut state: HashOutput = [0u64; 8];
    let mut input = [0u8; 128];
    // input[0..8].clone_from_slice(&.to_le_bytes());
    input[8..40].clone_from_slice(&opening.to_bytes());

    let blocks: &GenericArray<_, U128> = GenericArray::from_slice(&input); //[c.to_bytes(), pad].iter().flatten().into();
    compress512(&mut state, &[*blocks]);
    state
}

// FIXME this should be bit difficulty, not byte difficulty
pub(crate) fn pow_pass(hash_output: &HashOutput, difficulty: usize) -> bool {
    hash_output
        .iter()
        .map(|x| x.to_le_bytes())
        .flatten()
        .take(difficulty)
        .all(|x| x == 0)
}

#[cfg(test)]
mod tests {
    use kzg::types::fr::FsFr;

    use super::*;

    #[test]
    fn test_derive_indices() {
        let i = 1;
        let c = FsFr::one();
        let m = 8;
        let indices = derive_indices(i, &c, m);
        assert_eq!(indices.len(), m);
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
        hash_output[1] = u64::MAX;
        assert!(pow_pass(&hash_output, 8));
        assert!(!pow_pass(&hash_output, 9));
        hash_output[0] = u64::MAX;
        assert!(pow_pass(&hash_output, 0));
        assert!(!pow_pass(&hash_output, 1));
        hash_output[0] = (u8::MAX as u64) + 1;
        assert!(pow_pass(&hash_output, 0));
        assert!(pow_pass(&hash_output, 1));
        assert!(!pow_pass(&hash_output, 2));
    }
}

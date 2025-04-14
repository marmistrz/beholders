use beholders::Proof;

const N_INDICES: usize = 8;
const DIFFICULTY: u32 = 16;
const MVALUE: usize = 16;

fn main() {
    let proof = Proof {
        base_proofs: vec![],
    };
    assert!(proof
        .verify(
            &Default::default(),
            &Default::default(),
            20,
            &Default::default(),
            DIFFICULTY,
            MVALUE,
        )
        .unwrap());
}

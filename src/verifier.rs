use beholders::Proof;

const N_INDICES: usize = 8;
const DIFFICULTY: u32 = 16;

fn main() {
    let proof = Proof::<N_INDICES> {
        base_proofs: vec![],
    };
    assert!(proof
        .verify(
            &Default::default(),
            &Default::default(),
            20,
            &Default::default(),
            DIFFICULTY,
        )
        .unwrap());
}

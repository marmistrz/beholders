use beholders::Proof;

const DIFFICULTY: u32 = 16;
const MVALUE: usize = 16;

fn main() {
    let proof = Proof {
        fisch_iters: vec![],
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

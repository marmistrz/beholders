use beholders::Proof;
use kzg::eip_7594::BlstBackend;

type Backend = BlstBackend;
const N_INDICES: usize = 8;
const BYTE_DIFFICULTY: usize = 2;

fn main() {
    let proof = Proof::<Backend, N_INDICES> {
        base_proofs: vec![],
    };
    assert!(proof
        .verify(
            &Default::default(),
            &Default::default(),
            20,
            &Default::default(),
            BYTE_DIFFICULTY,
        )
        .unwrap());
}

use beholders::Proof;
use kzg::eip_7594::BlstBackend;

type Backend = BlstBackend;
const NFISCH: usize = 8;

fn main() {
    let proof = Proof::<Backend, NFISCH> {
        base_proofs: vec![],
    };
    assert!(proof
        .verify(
            &Default::default(),
            &Default::default(),
            20,
            &Default::default()
        )
        .unwrap());
}

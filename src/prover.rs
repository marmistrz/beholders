use std::time::Instant;

use beholders::Proof;
use kzg::{eip_4844::load_trusted_setup_filename_rust, eip_7594::BlstBackend, types::fr::FsFr};
use kzg_traits::{EcBackend, FFTFr, FFTSettings, Fr, KZGSettings, Poly};

const TRUSTED_SETUP_FILE: &str = "trusted_setup.txt";

type Backend = BlstBackend;

fn main() {
    let start: Instant = Instant::now();
    let kzg_settings =
        load_trusted_setup_filename_rust(TRUSTED_SETUP_FILE).expect("loading trusted setup");
    let duration = start.elapsed();
    println!("Initialization time: {:?}", duration);
    println!("Proving...");
    let start: Instant = Instant::now();
    let data = [1, 2, 3, 4];
    let sk = FsFr::from_u64(2137);
    let _duration = Proof::<Backend, 8>::prove(&kzg_settings, sk, &data, 2)
        .expect("KZG error")
        .expect("Proof not found");
    let duration = start.elapsed();
    println!("Proving time: {:?}", duration);

    // let prover = Prover::<Backend>::new(trusted_setup).unwrap();
    // let duration = start.elapsed();

    // println!("Initialization time: {:?}", duration);
    // prover.prove(&data);
}

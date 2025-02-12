use std::time::Instant;

use kzg::{eip_4844::load_trusted_setup_filename_rust, eip_7594::BlstBackend, types::kzg_settings::FsKZGSettings};
use kzg_traits::{EcBackend, Poly};

const TRUSTED_SETUP_FILE: &str = "trusted_setup.txt";



struct Prover {
    trusted_setup: FsKZGSettings,
}

impl Prover {
    fn new() -> Result<Self, String> {
        let trusted_setup = load_trusted_setup_filename_rust(TRUSTED_SETUP_FILE)?;
        Ok(Self {trusted_setup})
    }

    fn prove<B: EcBackend>(&self, data: &[i32]) {
        println!("Hello, world!");
    }}

fn main() {
    let data = [1, 2, 3, 4];
    let start = Instant::now();
    let prover = Prover::new().unwrap();
    let duration = start.elapsed();

    println!("Initialization time: {:?}", duration);
}

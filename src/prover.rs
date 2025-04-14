use std::{fs, time::Instant};

use anyhow::Context;
use beholders::Proof;
use clap::Parser;
use kzg::{eip_4844::load_trusted_setup_filename_rust, types::fr::FsFr};
use kzg_traits::Fr;

const TRUSTED_SETUP_FILE: &str = "trusted_setup.txt";

const BIT_DIFFICULTY: u32 = 22;
const NFISCH: usize = 64;

#[derive(Parser)]
struct Cli {
    /// The path to the file containing the data
    #[arg(index = 1)]
    data: std::path::PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let data = fs::read(&args.data).context(format!("Unable to read file: {:?}", args.data))?;
    let data: &[u64] = bytemuck::try_cast_slice(&data).unwrap();
    println!("Num chunks: {}", data.len());
    let sk = FsFr::from_u64(2137);
    let mvalue = 16;

    let start: Instant = Instant::now();
    println!("Loading trusted setup...");
    let kzg_settings =
        load_trusted_setup_filename_rust(TRUSTED_SETUP_FILE).expect("loading trusted setup");
    let duration = start.elapsed();
    println!("Initialization time: {:?}", duration);

    println!("Proving...");
    let start: Instant = Instant::now();

    let _proof = Proof::prove(&kzg_settings, sk, data, NFISCH, BIT_DIFFICULTY, mvalue)
        .map_err(anyhow::Error::msg)
        .context("KZG error")?
        .context("Could not find solve the proof-of-work in the beholder signature")?;
    let duration = start.elapsed();
    println!("Proving time: {:?}", duration);
    // println!("Proof: {:?}", proof);

    Ok(())

    // let prover = Prover::<Backend>::new(trusted_setup).unwrap();
    // let duration = start.elapsed();

    // println!("Initialization time: {:?}", duration);
    // prover.prove(&data);
}

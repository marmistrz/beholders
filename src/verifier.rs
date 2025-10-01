use std::path::PathBuf;

use beholders::{
    commitment::{Commitment, TrustedSetup},
    hashing::difficulty,
    proof::CHUNK_SIZE,
    types::{TFr, TG1},
    util::{fft_settings, read_from_file},
    Proof,
};
use clap::Parser;
use kzg_traits::{Fr, G1Mul, G1};

#[derive(Parser)]
struct Cli {
    /// The path to the file containing the commitment
    #[arg(index = 1)]
    commitment: PathBuf,

    /// The path to the file containing the signature
    #[arg(index = 2)]
    signature: PathBuf,

    /// The numeber of indices to derive for each Schnorr transcript
    #[arg(long, default_value_t = 16)]
    mvalue: usize,

    /// The difficulty of the proof-of-work
    /// (default is log2(N) + 3)
    #[arg(long)]
    bit_difficulty: Option<u32>,

    /// Length of the data, in bytes.
    #[arg(long)]
    data_len: usize,

    /// Location of the trusted setup file.
    #[arg(long)]
    setup_file: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let sk = TFr::from_u64(2137);
    let pk = TG1::generator().mul(&sk);

    let chunks = args.data_len / CHUNK_SIZE;
    let bit_difficulty = args.bit_difficulty.unwrap_or_else(|| difficulty(chunks));

    println!("Loading trusted setup");
    let trusted_setup: TrustedSetup = read_from_file(&args.setup_file)?;
    let fs = fft_settings(chunks).map_err(anyhow::Error::msg)?;
    let kzg_settings = trusted_setup
        .into_kzg_settings(&fs)
        .map_err(anyhow::Error::msg)?;

    println!("Done loading trusted setup");

    let proof: Proof = read_from_file(&args.signature)?;
    let commitment: Commitment = read_from_file(&args.commitment)?;

    proof
        .verify(
            &pk,
            &commitment,
            chunks,
            &kzg_settings,
            bit_difficulty,
            args.mvalue,
        )
        .expect("KZG error");

    println!("Proof verified successfully");
    Ok(())
}

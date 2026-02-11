use std::path::PathBuf;
use std::time::Instant;

use anyhow::bail;
use beholders::{
    commitment::{Commitment, TrustedSetup},
    hashing::difficulty,
    proof::CHUNK_SIZE,
    schnorr::PublicKey,
    util::{fft_settings, read_from_file},
    Proof,
};
use clap::Parser;

#[derive(Parser)]
struct Cli {
    /// The path to the file containing the commitment
    #[arg(index = 1)]
    commitment: PathBuf,

    /// The path to the file containing the signature
    #[arg(index = 2)]
    signature: PathBuf,

    /// The number of indices to derive for each Schnorr transcript
    #[arg(long, default_value_t = 6)]
    mvalue: usize,

    /// The number of Fischlin iterations parameter (default: 10)
    #[arg(long, default_value_t = 10)]
    nfisch: usize,

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

    /// Path for the public key.
    #[arg(long)]
    public_key: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let pk: PublicKey = read_from_file(&args.public_key)?;

    let chunks = args.data_len / CHUNK_SIZE;
    let nfisch = args.nfisch;
    let bit_difficulty = args
        .bit_difficulty
        .unwrap_or_else(|| difficulty(chunks, nfisch));


    println!("Loading trusted setup");
    let setup_start = Instant::now();
    let trusted_setup: TrustedSetup = read_from_file(&args.setup_file)?;
    let fs = fft_settings(chunks).map_err(anyhow::Error::msg)?;
    let kzg_settings = trusted_setup
        .into_kzg_settings(&fs)
        .map_err(anyhow::Error::msg)?;
    let setup_duration = setup_start.elapsed();
    println!("Done loading trusted setup (took: {:.3?})", setup_duration);

    let proof: Proof = read_from_file(&args.signature)?;
    let commitment: Commitment = read_from_file(&args.commitment)?;

    let verify_start = Instant::now();
    let output = proof
        .verify(
            &pk,
            &commitment,
            chunks,
            &kzg_settings,
            bit_difficulty,
            args.mvalue,
        )
        .expect("KZG error");
    let verify_duration = verify_start.elapsed();
    println!("Verification took: {:.3?}", verify_duration);
    match output {
        true => {
            println!("Proof verified successfully");
            Ok(())
        }
        false => {
            bail!("Proof verification failed")
        }
    }
}

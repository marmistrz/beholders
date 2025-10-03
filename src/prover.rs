use std::{fs, path::PathBuf, time::Instant};

use anyhow::{bail, Context};
use beholders::{
    commitment::TrustedSetup,
    hashing::difficulty,
    proof::CHUNK_SIZE,
    schnorr::SecretKey,
    util::{fft_settings, read_from_file, write_to_file},
    Proof,
};
use clap::Parser;
use humansize::{format_size, BINARY};

// const TRUSTED_SETUP_FILE: &str = "trusted_setup.txt"; // TRUSTED SETUP

#[derive(Parser)]
struct Cli {
    /// The path to the file containing the data
    #[arg(index = 1)]
    data: std::path::PathBuf,

    /// The path where the commitment should be written
    #[arg(index = 2)]
    commitment: std::path::PathBuf,

    /// The signature output path
    #[arg(index = 3)]
    signature: std::path::PathBuf,

    /// The number of indices to derive for each Schnorr transcript
    #[arg(long, default_value_t = 6)]
    mvalue: usize,

    /// The number of Fischlin iterations parameter (default: 10)
    #[arg(long, default_value_t = 10)]
    nfisch: usize,

    /// The difficulty of the proof-of-work
    /// (default is 5 + log2(N) - log2(nfisch)),
    /// where N is the length in chunks of 32 bytes
    #[arg(long)]
    bit_difficulty: Option<u32>,

    /// Location of the trusted setup file.
    #[arg(long)]
    setup_file: PathBuf,

    /// Path for the secret key.
    #[arg(long)]
    secret_key: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let data = fs::read(&args.data).context(format!("Unable to read file: {:?}", args.data))?;
    if !data.len().is_power_of_two() {
        bail!("Data length needs to be a power of two");
    }

    let mvalue = args.mvalue;
    let nfisch = args.nfisch;

    println!("File size: {}", format_size(data.len(), BINARY));
    let chunks = data.len() / CHUNK_SIZE;
    println!("Num chunks: {chunks}");
    let bit_difficulty = args
        .bit_difficulty
        .unwrap_or_else(|| difficulty(chunks, nfisch));

    let sk: SecretKey = read_from_file(&args.secret_key)?;

    println!(
        "Parameters: nfisch: {}, d: {}, m: {}",
        nfisch, bit_difficulty, mvalue
    );

    let start: Instant = Instant::now();

    println!("Loading trusted setup...");
    let fs = fft_settings(chunks).map_err(anyhow::Error::msg)?;
    let trusted_setup: TrustedSetup = read_from_file(&args.setup_file)?;

    println!(
        "Trusted setup: {} {} {}",
        trusted_setup.g1_monomial.len(),
        trusted_setup.g1_lagrange.len(),
        trusted_setup.g2_monomial.len()
    );

    println!("Building KZG settings...");
    let kzg_settings = trusted_setup
        .into_kzg_settings(&fs)
        .map_err(anyhow::Error::msg)
        .context("Loading trusted setup")?;
    let duration = start.elapsed();
    println!("Initialization time: {:?}", duration);

    println!("Proving...");
    let start: Instant = Instant::now();

    let (proof, com) = Proof::prove(&kzg_settings, sk, &data, nfisch, bit_difficulty, mvalue)
        .map_err(anyhow::Error::msg)
        .context("KZG error")?;
    let proof =
        proof.context("Could not find solve the proof-of-work in the beholder signature")?;
    let duration = start.elapsed();
    println!("Proving time: {:?}", duration);

    write_to_file(&args.commitment, &com)?;
    write_to_file(&args.signature, &proof)?;

    Ok(())

    // let prover = Prover::<Backend>::new(trusted_setup).unwrap();
    // let duration = start.elapsed();

    // println!("Initialization time: {:?}", duration);
    // prover.prove(&data);
}

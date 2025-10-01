use std::{fs, path::PathBuf, time::Instant};

use anyhow::{bail, Context};
use beholders::{
    commitment::TrustedSetup,
    hashing::difficulty,
    proof::CHUNK_SIZE,
    util::{fft_settings, read_from_file, write_to_file},
    Proof,
};
use clap::Parser;
use humansize::{format_size, BINARY};
use kzg::{
    // eip_4844::load_trusted_setup_filename_rust, // TRUSTED SETUP
    types::fr::FsFr,
};
use kzg_traits::Fr;

// const TRUSTED_SETUP_FILE: &str = "trusted_setup.txt"; // TRUSTED SETUP

const NFISCH: usize = 10;

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
    #[arg(long, default_value_t = 4)]
    mvalue: usize,

    /// The difficulty of the proof-of-work
    /// (default is log2(N) + 3)
    /// where N is the length in chunks of 32 bytes
    #[arg(long)]
    bit_difficulty: Option<u32>,

    /// Location of the trusted setup file.
    #[arg(long)]
    setup_file: PathBuf,

    /// Secret key as 32-byte hex string (big-endian). Random if not provided.
    #[arg(long)]
    secret_key: Option<String>, // Added secret-key option
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let data = fs::read(&args.data).context(format!("Unable to read file: {:?}", args.data))?;
    if !data.len().is_power_of_two() {
        bail!("Data length needs to be a power of two");
    }

    let mvalue = args.mvalue;

    println!("File size: {}", format_size(data.len(), BINARY));
    let chunks = data.len() / CHUNK_SIZE;
    println!("Num chunks: {chunks}");
    let bit_difficulty = args.bit_difficulty.unwrap_or_else(|| difficulty(chunks));

    // Handle secret key input (added logic)
    let sk = if let Some(hex_sk) = &args.secret_key {
        // Strip optional 0x prefix
        let hex_str = hex_sk.strip_prefix("0x").unwrap_or(hex_sk);
        let bytes = hex::decode(hex_str).context("Failed to decode hex secret key")?;

        // Validate length
        if bytes.len() != 32 {
            bail!("Secret key must be 32 bytes, got {}", bytes.len());
        }

        // Convert big-endian input to little-endian
        let mut le_bytes = bytes.clone();
        le_bytes.reverse();
        let mut array = [0u8; 32];
        array.copy_from_slice(&le_bytes);

        FsFr::from_bytes(&array).map_err(|e| anyhow::anyhow!("Invalid secret key: {}", e))?
    } else {
        // Fallback to random generation
        FsFr::rand()
    };

    println!(
        "Parameters: nfisch: {}, d: {}, m: {}",
        NFISCH, bit_difficulty, mvalue
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

    let (proof, com) = Proof::prove(&kzg_settings, sk, &data, NFISCH, bit_difficulty, mvalue)
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

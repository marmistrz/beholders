use std::{fs, time::Instant};

use anyhow::{bail, Context};
use beholders::Proof;
use clap::Parser;
use humansize::{format_size, BINARY};
use kzg::{
    // eip_4844::load_trusted_setup_filename_rust, // TRUSTED SETUP
    types::{
        fft_settings::FsFFTSettings,
        fr::FsFr,
        kzg_settings::FsKZGSettings, // TRUSTED SETUP
    },
    utils::generate_trusted_setup,
};
use kzg_traits::{FFTSettings, Fr, KZGSettings};

// const TRUSTED_SETUP_FILE: &str = "trusted_setup.txt"; // TRUSTED SETUP

const NFISCH: usize = 10;
#[derive(Parser)]
struct Cli {
    /// The path to the file containing the data
    #[arg(index = 1)]
    data: std::path::PathBuf,

    /// The numeber of indices to derive for each Schnorr transcript
    #[arg(long, default_value_t = 4)]
    mvalue: usize,

    /// The difficulty of the proof-of-work
    /// (default is log2(N) + 3)
    /// where N is the length in chunks of 32 bytes
    #[arg(long)]
    bit_difficulty: Option<u32>,

    /// Secret key as 32-byte hex string (big-endian). Random if not provided.
    #[arg(long)]
    secret_key: Option<String>, // Added secret-key option
}

fn difficulty(data_len: usize) -> u32 {
    data_len.ilog2()
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let data = fs::read(&args.data).context(format!("Unable to read file: {:?}", args.data))?;
    if !data.len().is_power_of_two() {
        bail!("Data length needs to be a power of two");
    }

    let mvalue = args.mvalue;

    // let data: &[u64] = bytemuck::try_cast_slice(&data).unwrap();
    println!("File size: {}", format_size(data.len(), BINARY));
    let chunks = data.len() / 32;
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

    // Data has 2^{scale-1} chunks of 32 bytes
    let secrets_exp = chunks.ilog2();
    let scale = secrets_exp + 1;
    println!("Generating trusted setup, 2^{secrets_exp} secrets, FFT scale={scale}...");
    let secrets_len = chunks;
    let (s1, s2, s3) = generate_trusted_setup(secrets_len, [0; 32]);
    let fs = FsFFTSettings::new(scale as usize).unwrap();
    let kzg_settings =
        FsKZGSettings::new(&s1, &s2, &s3, &fs, kzg_traits::eth::FIELD_ELEMENTS_PER_CELL).unwrap();
    // let kzg_settings =
    //     load_trusted_setup_filename_rust(TRUSTED_SETUP_FILE).expect("loading trusted setup");
    let duration = start.elapsed();
    println!("Initialization time: {:?}", duration);

    println!("Proving...");
    let start: Instant = Instant::now();

    let _proof = Proof::prove(&kzg_settings, sk, &data, NFISCH, bit_difficulty, mvalue)
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

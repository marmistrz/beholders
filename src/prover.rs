use std::{
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::PathBuf,
    time::Instant,
};

use anyhow::{bail, Context};
use beholders::{commitment::TrustedSetup, proof::CHUNK_SIZE, Proof};
use clap::Parser;
use humansize::{format_size, BINARY};
use kzg::{
    // eip_4844::load_trusted_setup_filename_rust, // TRUSTED SETUP
    types::{fft_settings::FsFFTSettings, fr::FsFr},
};
use kzg_traits::{FFTSettings, Fr};

// const TRUSTED_SETUP_FILE: &str = "trusted_setup.txt"; // TRUSTED SETUP

const NFISCH: usize = 10;
#[derive(Parser)]
struct Cli {
    /// The path to the file containing the data
    #[arg(index = 1)]
    data: std::path::PathBuf,

    /// The numeber of indices to derive for each Schnorr transcript
    #[arg(long, default_value_t = 16)]
    mvalue: usize,

    /// The difficulty of the proof-of-work
    /// (default is log2(data_len) + 3)
    #[arg(long)]
    bit_difficulty: Option<u32>,

    /// Location of the trusted setup file.
    #[arg(long)]
    setup_file: PathBuf,
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
    let bit_difficulty = args
        .bit_difficulty
        .unwrap_or_else(|| difficulty(data.len()));
    let mvalue = args.mvalue;

    println!("File size: {}", format_size(data.len(), BINARY));
    let chunks = data.len() / CHUNK_SIZE;
    println!("Num chunks: {chunks}");
    let sk = FsFr::from_u64(2137);
    println!(
        "Parameters: nfisch: {}, d: {}, m: {}",
        NFISCH, bit_difficulty, mvalue
    );

    let start: Instant = Instant::now();

    // Data has 2^{scale-1} chunks of 32 bytes
    let secrets_exp = chunks.ilog2();
    let scale = secrets_exp + 1;

    println!("Loading trusted setup, 2^{secrets_exp} secrets, FFT scale={scale}...");
    let fs: FsFFTSettings = FsFFTSettings::new(scale as usize).unwrap();
    let file = File::open(&args.setup_file)
        .context(format!("Unable to open file: {:?}", args.setup_file))?;
    let mut reader = BufReader::new(file);
    let trusted_setup: TrustedSetup =
        bincode::serde::decode_from_std_read(&mut reader, bincode::config::standard())
            .context("Reading trusted setup")?;
    println!(
        "Trusted setup: {} {} {}",
        trusted_setup.g1_monomial.len(),
        trusted_setup.g1_lagrange.len(),
        trusted_setup.g2_monomial.len()
    );

    let kzg_settings = trusted_setup
        .into_kzg_settings(&fs)
        .map_err(anyhow::Error::msg)
        .context("Loading trusted setup")?;
    let duration = start.elapsed();
    println!("Initialization time: {:?}", duration);

    println!("Proving...");
    let start: Instant = Instant::now();

    let proof = Proof::prove(&kzg_settings, sk, &data, NFISCH, bit_difficulty, mvalue)
        .map_err(anyhow::Error::msg)
        .context("KZG error")?
        .context("Could not find solve the proof-of-work in the beholder signature")?;
    let duration = start.elapsed();
    println!("Proving time: {:?}", duration);

    let file = File::create("proof.bin").expect("Unable to create file");
    let mut writer = BufWriter::new(file);
    bincode::serde::encode_into_std_write(&proof, &mut writer, bincode::config::standard())
        .expect("Serialization failed");

    Ok(())

    // let prover = Prover::<Backend>::new(trusted_setup).unwrap();
    // let duration = start.elapsed();

    // println!("Initialization time: {:?}", duration);
    // prover.prove(&data);
}

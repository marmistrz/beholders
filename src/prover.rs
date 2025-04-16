use std::{fs, time::Instant};

use anyhow::Context;
use beholders::Proof;
use clap::Parser;
use kzg::{
    eip_4844::load_trusted_setup_filename_rust, // TRUSTED SETUP
    types::{
        fft_settings::FsFFTSettings,
        fr::FsFr,
        kzg_settings::{self, FsKZGSettings}, // TRUSTED SETUP
    },
    utils::generate_trusted_setup,
};
use kzg_traits::{FFTSettings, Fr, KZGSettings};

const TRUSTED_SETUP_FILE: &str = "trusted_setup.txt"; // TRUSTED SETUP

const BIT_DIFFICULTY: u32 = 15;
const NFISCH: usize = 10;

#[derive(Parser)]
struct Cli {
    /// The path to the file containing the data
    #[arg(index = 1)]
    data: std::path::PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let rawdata = fs::read(&args.data).context(format!("Unable to read file: {:?}", args.data))?;
    let data = &rawdata[..rawdata.len() / 1];
    // let data: &[u64] = bytemuck::try_cast_slice(&data).unwrap();
    println!("Num chunks: {}", data.len() / 32);
    let sk = FsFr::from_u64(2137);
    let mvalue = 16;
    println!(
        "Parameters: nfish: {}, d: {}, m: {}",
        NFISCH, BIT_DIFFICULTY, mvalue
    );

    let start: Instant = Instant::now();
    println!("Generating trusted setup...");

    // Data has 2^{scale-1} chunks of 32 bytes
    let scale = 17;
    let secrets_len = 2_usize.pow(scale - 1);
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

    let _proof = Proof::prove(&kzg_settings, sk, &data, NFISCH, BIT_DIFFICULTY, mvalue)
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

use std::{fs::File, io::BufWriter, path::PathBuf};

use anyhow::Context;
use beholders::types::TFr;
use clap::Parser;
use kzg::types::g1::FsG1;
use kzg_traits::{Fr, G1Mul, G1};

#[derive(Parser)]
struct Cli {
    /// The path where the secret key will be written
    #[arg(long)]
    secret_key: PathBuf,

    /// The path where the public key will be written
    #[arg(long)]
    public_key: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let sk = TFr::rand();
    let pk = FsG1::generator().mul(&sk);

    let file = File::create(args.secret_key).context("Unable to create file")?;
    let mut writer = BufWriter::new(file);
    bincode::serde::encode_into_std_write(sk, &mut writer, bincode::config::standard())
        .context("Writing trusted setup")?;

    let file = File::create(args.public_key).context("Unable to create file")?;
    let mut writer = BufWriter::new(file);
    bincode::serde::encode_into_std_write(pk, &mut writer, bincode::config::standard())
        .context("Writing trusted setup")?;

    Ok(())
}

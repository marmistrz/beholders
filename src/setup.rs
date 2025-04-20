use std::{fs::File, io::BufWriter};

use anyhow::Context;
use clap::Parser;
use kzg::{
    types::{g1::FsG1, g2::FsG2, kzg_settings::FsKZGSettings},
    utils::generate_trusted_setup,
};
use kzg_traits::eth;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct TrustedSetup {
    g1_monomial: Vec<FsG1>,
    g1_lagrange: Vec<FsG1>,
    g2_monomial: Vec<FsG2>,
}

impl TrustedSetup {
    pub fn from_kzg_settings(kzg_settings: FsKZGSettings) -> Self {
        Self {
            g1_monomial: kzg_settings.g1_values_monomial,
            g1_lagrange: kzg_settings.g1_values_lagrange_brp,
            g2_monomial: kzg_settings.g2_values_monomial,
        }
    }
}

#[derive(Parser)]
struct Cli {
    /// The path to the file containing the data
    #[arg(index = 1)]
    output: std::path::PathBuf,

    /// The number of secrets to generate
    #[arg(long)]
    secrets: usize,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let secrets_len = args.secrets;
    assert!(
        secrets_len.is_power_of_two(),
        "Secrets length needs to be a power of two",
    );
    assert!(
        secrets_len >= eth::FIELD_ELEMENTS_PER_CELL,
        "Secrets length needs to be at least {}",
        eth::FIELD_ELEMENTS_PER_CELL
    );

    let g2_len = eth::TRUSTED_SETUP_NUM_G2_POINTS;
    let (g1_monomial, g1_lagrange, mut g2_monomial) = generate_trusted_setup(secrets_len, [1; 32]);
    g2_monomial.truncate(g2_len);
    let setup = TrustedSetup {
        g1_monomial,
        g1_lagrange,
        g2_monomial,
    };

    let file = File::create(args.output).context("Unable to create file")?;
    let mut writer = BufWriter::new(file);
    bincode::serde::encode_into_std_write(&setup, &mut writer, bincode::config::standard())
        .context("Writing trusted setup")?;

    Ok(())
}

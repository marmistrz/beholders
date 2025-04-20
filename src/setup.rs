use std::{fs::File, io::BufWriter};

use anyhow::Context;
use beholders::commitment::TrustedSetup;
use clap::Parser;
use kzg::utils::generate_trusted_setup;
use kzg_traits::eth;

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

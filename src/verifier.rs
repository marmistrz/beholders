use std::path::PathBuf;

use beholders::Proof;
use clap::Parser;

const DIFFICULTY: u32 = 16;
const MVALUE: usize = 16;

#[derive(Parser)]
struct Cli {
    /// The path to the file containing the data
    #[arg(index = 1)]
    signature: PathBuf,

    /// The numeber of indices to derive for each Schnorr transcript
    #[arg(long, default_value_t = 16)]
    mvalue: usize,

    /// The difficulty of the proof-of-work
    /// (default is log2(data_len) + 3)
    #[arg(long)]
    bit_difficulty: Option<u32>,

    // TODO
    data_len: usize,

    /// Location of the trusted setup file.
    #[arg(long)]
    setup_file: PathBuf,
}

fn main() {
    let args = Cli::parse();

    let proof = Proof {
        base_proofs: vec![],
    };
    proof
        .verify(
            &Default::default(),
            &Default::default(),
            args.data_len,
            &Default::default(),
            DIFFICULTY,
            args.mvalue,
        )
        .unwrap();
}

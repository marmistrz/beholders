use std::time::Duration;

use beholders::{commitment::open_all, proof::BaseProof};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kzg::{eip_4844::load_trusted_setup_filename_rust, eip_7594::BlstBackend, types::fr::FsFr};
use kzg_traits::Fr;

type Backend = BlstBackend;
const M: usize = 8;
const TRUSTED_SETUP_FILE: &str = "trusted_setup.txt";

pub fn criterion_benchmark(c: &mut Criterion) {
    let data = (0..128).collect::<Vec<u64>>();

    let kzg_settings =
        load_trusted_setup_filename_rust(TRUSTED_SETUP_FILE).expect("loading trusted setup");

    // Benchmark the precomputation (opening all data for the KZG commitment)
    c.bench_function("open_all", |b| {
        b.iter(|| open_all::<Backend>(&kzg_settings, black_box(&data)).expect("KZG error"))
    });

    let sk = FsFr::from_u64(2137);
    let r = FsFr::from_u64(1337);
    let bit_difficulty = 14;
    let openings = open_all::<Backend>(&kzg_settings, &data).expect("openings");
    assert_eq!(openings.len(), data.len());

    let fisch_iter = 0;
    let prelude = [0; 8];

    // Benchmark the Fischlin Mining
    c.bench_function("mining", |b| {
        b.iter(|| {
            BaseProof::<Backend, M>::prove(
                black_box(fisch_iter),
                black_box(prelude),
                black_box(&openings),
                black_box(&r),
                black_box(&sk),
                black_box(&data),
                bit_difficulty,
            )
            .expect("No proof found");
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(15));
    targets = criterion_benchmark
}
criterion_main!(benches);

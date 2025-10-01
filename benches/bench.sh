#!/bin/bash
set -e

# Prechecks
for size in 128 256 512 1024 2048; do
  if ! [ -f data$size.bin ]; then
    echo "Benchmark data missing, run gen.sh first"
    exit 1
  fi
done
mkdir res || (echo "Refusing to overwrite existing results" && exit 1)

# Run benchmarks
for size in 128 256 512 1024 2048; do
  echo "size=$size KiB"
  cargo run --bin prover --release -- data$size.bin | tee "res/out$size.txt"
done

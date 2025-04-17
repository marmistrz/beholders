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
for d in {14..19}; do
  for size in 128 256 512 1024 2048; do
    echo "difficulty=$d, size=$size KiB"
    cargo run --bin prover --release -- --bit-difficulty "$d" data$size.bin | tee "res/out$size-$d.txt"
  done
done

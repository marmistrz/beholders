#!/bin/bash
set -e

# Parse arguments
overwrite=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    '--overwrite')
      overwrite=true
      shift
      ;;
    *)
      shift
      ;;
  esac
done

# Remove res directory if --overwrite is specified
if $overwrite; then
  echo "Removing existing results"
  rm -rf res
fi

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
  size_bytes=$((size * 1024))
  N=$((size_bytes/32))
  cargo run --bin prover --release -- --setup-file secrets$N.bin data$size.bin com.bin sig.bin | tee "res/out$size.txt"
done

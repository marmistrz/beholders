#!/bin/bash
set -e



# Parse arguments
overwrite=false
num_iterations=""
show_help() {
  echo "Usage: $0 --num-iterations N [--overwrite]"
  echo "  --num-iterations N   Required. Number of iterations for each benchmark."
  echo "  --overwrite          Optional. Remove existing results before running."
  echo "  --help               Show this help message."
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    '--overwrite')
      overwrite=true
      shift
      ;;
    '--num-iterations')
      num_iterations="$2"
      shift 2
      ;;
    '--help')
      show_help
      exit 0
      ;;
    *)
      show_help
      exit 1
      ;;
  esac
done

if [[ -z "$num_iterations" ]]; then
  echo "Error: --num-iterations is required."
  show_help
  exit 1
fi

if ! [[ "$num_iterations" =~ ^[0-9]+$ ]]; then
  echo "Error: --num-iterations must be a number."
  show_help
  exit 1
fi

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
  for it in $(seq 1 $num_iterations); do
    echo "Iteration $it/$num_iterations for size $size KiB"

    size_bytes=$((size * 1024))
    N=$((size_bytes/32))
    cargo run --bin prover --release -- --setup-file secrets$N.bin data$size.bin com.bin sig.bin | tee "res/out$size-$it.txt"
  done
done

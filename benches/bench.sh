#!/bin/bash
set -e



# Parse arguments
overwrite=false
continue_mode=false
num_iterations=""
show_help() {
  echo "Usage: $0 --num-iterations N [--overwrite | --continue]"
  echo "  --num-iterations N   Required. Number of iterations for each benchmark."
  echo "  --overwrite          Optional. Remove existing results before running. Exclusive with --continue."
  echo "  --continue           Optional. Resume benchmarks, skip runs if output file exists. Exclusive with --overwrite."
  echo "  --help               Show this help message."
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    '--overwrite')
      if $continue_mode; then
        echo "Error: --overwrite and --continue cannot be used together."
        show_help
        exit 1
      fi
      overwrite=true
      shift
      ;;
    '--continue')
      if $overwrite; then
        echo "Error: --overwrite and --continue cannot be used together."
        show_help
        exit 1
      fi
      continue_mode=true
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
for size in 16 32 64 128 256 512 1024 2048; do
  if ! [ -f data$size.bin ]; then
    echo "Benchmark data missing, run gen.sh first"
    exit 1
  fi
done


# Handle res directory existence
if ! $overwrite && ! $continue_mode && [ -d res ]; then
  echo "Error: res directory already exists. Use --overwrite to remove it or --continue to resume."
  exit 1
fi
if ! [ -d res ]; then
  mkdir res
fi

# Run benchmarks
for size in 128 256 512 1024 2048; do
  for it in $(seq 1 $num_iterations); do
    out_file="res/out${size}-${it}.txt"
    if $continue_mode && [ -f "$out_file" ]; then
      echo "Skipping iteration $it/$num_iterations for size $size KiB (output exists)"
      continue
    fi
    echo "Iteration $it/$num_iterations for size $size KiB"

    size_bytes=$((size * 1024))
    N=$((size_bytes/32))

    # Trap SIGINT to delete output file if interrupted
    trap 'echo "Interrupted, deleting $out_file"; rm -f "$out_file"; exit 130' INT
    cargo run --bin prover --release -- --setup-file secrets$N.bin data$size.bin com.bin sig.bin | tee "$out_file"
    trap - INT
  done
done

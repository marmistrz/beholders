#!/bin/bash
set -e



# Parse arguments
overwrite=false
continue_mode=false
num_iterations=""
prover_mode=false
verifier_mode=false
show_help() {
  echo "Usage: $0 --num-iterations N (--prover | --verifier) [--overwrite | --continue]"
  echo "  --num-iterations N   Required. Number of iterations for each benchmark."
  echo "  --prover             Benchmark the prover. Exclusive with --verifier. Required."
  echo "  --verifier           Benchmark the verifier. Exclusive with --prover. Required."
  echo "  --overwrite          Optional. Remove existing results before running. Exclusive with --continue."
  echo "  --continue           Optional. Resume benchmarks, skip runs if output file exists. Exclusive with --overwrite."
  echo "  --help               Show this help message."
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    '--prover')
      if $verifier_mode; then
        echo "Error: --prover and --verifier cannot be used together."
        show_help
        exit 1
      fi
      prover_mode=true
      shift
      ;;
    '--verifier')
      if $prover_mode; then
        echo "Error: --prover and --verifier cannot be used together."
        show_help
        exit 1
      fi
      verifier_mode=true
      shift
      ;;
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

if ! $prover_mode && ! $verifier_mode; then
  echo "Error: Either --prover or --verifier must be specified."
  show_help
  exit 1
fi

if ! [[ "$num_iterations" =~ ^[0-9]+$ ]]; then
  echo "Error: --num-iterations must be a number."
  show_help
  exit 1
fi

if $prover_mode; then
  echo "Running prover benchmarks with $num_iterations iterations each"
else
  echo "Running verifier benchmarks with $num_iterations iterations each"
fi

# Set output directory based on mode
if $prover_mode; then
  outdir="prover"
else
  outdir="verifier"
fi

# Remove output directory if --overwrite is specified
if $overwrite; then
  echo "Removing existing results in $outdir"
  rm -rf "$outdir"
fi

# Prechecks
for size in 16 32 64 128 256 512 1024 2048; do
  if ! [ -f data$size.bin ]; then
    echo "Benchmark data missing, run gen.sh first"
    exit 1
  fi
  if $verifier_mode; then
    if ! [ -f sig$size.bin ]; then
      echo "Signature file sig$size.bin missing for verifier mode. Run prover benchmarks first."
      exit 1
    fi
    if ! [ -f com$size.bin ]; then
      echo "Commitment file com$size.bin missing for verifier mode. Run prover benchmarks first."
      exit 1
    fi
  fi
done



# Handle output directory existence
if ! $overwrite && ! $continue_mode && [ -d "$outdir" ]; then
  echo "Error: $outdir directory already exists. Use --overwrite to remove it or --continue to resume."
  exit 1
fi
if ! [ -d "$outdir" ]; then
  mkdir "$outdir"
fi

# Run benchmarks
for size in 16 32 64 128 256 512 1024 2048; do
  for it in $(seq 1 $num_iterations); do
    out_file="$outdir/out${size}-${it}.txt"
    if $continue_mode && [ -f "$out_file" ]; then
      echo "Skipping iteration $it/$num_iterations for size $size KiB (output exists)"
      continue
    fi
    echo "Iteration $it/$num_iterations for size $size KiB"

    size_bytes=$((size * 1024))
    N=$((size_bytes/32))

    # Trap SIGINT to delete output file if interrupted
    trap 'echo "Interrupted, deleting $out_file"; rm -f "$out_file"; exit 130' INT
    if $prover_mode; then
      cargo run --bin prover --release -- --secret-key=sk.bin --setup-file secrets$N.bin data$size.bin com$size.bin sig$size.bin | tee "$out_file"
    else
      cargo run --bin verifier --release -- --setup-file secrets$N.bin --data-len "$size_bytes" --public-key=pk.bin com$size.bin sig$size.bin | tee "$out_file"
    fi
    trap - INT
    
    if [ $it -ne $num_iterations ]; then
      # Sleep to mitigate thermal throttling between runs 
      delay=$((size/64 + 5))
      echo "Sleeping ${delay}s"
      sleep ${delay}s 
    fi
  done
done

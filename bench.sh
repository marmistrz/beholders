#!/bin/bash
set -x
set -e

mkdir res
for d in {14..19}; do
  for size in 128 256 512 1024 2048; do
    echo "difficulty=$d, size=$size KiB"
    cargo run --bin prover --release -- --bit-difficulty "$d" data$size.bin | tee "res/out$size-$d.txt"
  done
done

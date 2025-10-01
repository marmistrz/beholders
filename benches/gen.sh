#!/bin/bash
set -e
# Generate benchmark
echo "==== Generating benchmark files ===="
for size in 128 256 512 1024 2048; do
    count=$((size/4))
    echo "Generating $size KiB of random data, $count blocks of 4 KiB"
    dd if=/dev/random of=data$size.bin bs=4K count=$count
done

echo "==== Generating trusted setup ===="
for size in 128 256 512 1024 2048; do
    size_bytes=$((size * 1024))
    N=$((size_bytes/32))
    echo "Generating trusted setup for $N chunks"
    cargo run --bin setup --release -- --secrets $N secrets$N.bin
done
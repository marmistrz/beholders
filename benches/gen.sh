#!/bin/bash
for size in 128 256 512 1024 2048; do
    count=$((size/4))
    echo "Generating $size KiB of random data, $count blocks of 4 KiB"
    dd if=/dev/random of=data$size.bin bs=4K count=$count
done
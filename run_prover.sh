#!/bin/bash

# Clear existing results file
> results.txt

# Get the full size of the original file
if [[ "$OSTYPE" == "darwin"* ]]; then
    full_size=$(stat -f%z data.bin)
else
    full_size=$(stat -c%s data.bin 2>/dev/null || wc -c < data.bin | tr -d ' ')
fi
echo "Original data.bin size: $full_size bytes"

# Define fractions with their string representations
declare -A fractions=(
    [1048576]="full"  # Full size
    [524288]="1/2"
    [262144]="1/4"
    [131072]="1/8"
    [65536]="1/16"
)

# Create fractional files and run prover
for fraction_size in "${!fractions[@]}"; do
    fraction=${fractions[$fraction_size]}
    
    # Verify the size is valid
    if (( fraction_size > full_size )); then
        echo "Fraction size $fraction_size is larger than original $full_size, skipping"
        continue
    fi
    
    # Create filename for this fraction
    fraction_file="data_${fraction_size}.bin"
    
    # Create fractional file
    echo "Creating $fraction_file ($fraction_size bytes) representing $fraction of original..."
    
    # Use head or dd to create the file
    if command -v head &> /dev/null; then
        head -c "$fraction_size" data.bin > "$fraction_file"
    else
        dd if=data.bin of="$fraction_file" bs=1 count="$fraction_size" status=none
    fi
    
    # Run prover for each bit difficulty
    for bit_difficulty in {13..18}; do
        echo "Running with $fraction fraction (${fraction_size} bytes), bit_difficulty=$bit_difficulty"
        
        # Run the prover and capture its output to results.txt
        cargo run --bin prover -- "$fraction_file" --bit-difficulty "$bit_difficulty" 2>&1 | tee -a results.txt
        
        echo "---------------------------------------------"
    done
    
    # Clean up fractional file
    rm "$fraction_file"
done

echo "All tests completed. Prover results saved to results.txt"
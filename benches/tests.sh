#!/bin/bash

# Exit immediately on error
set -e

# Check if results.txt exists and abort if it does
if [ -f "results.txt" ]; then
    echo "Error: results.txt already exists. Aborting to prevent overwrite."
    exit 1
fi

# Set data file name
data_file="data32KB.bin"

# Hardcoded valid secret key (big-endian hex format)
# This key represents a valid scalar value for the BLS12-381 curve
# Equivalent to decimal value 1 (0x0000...0001)
secret_key="0000000000000000000000000000000000000000000000000000000000000001"
echo "Using hardcoded valid secret key: $secret_key"

# Set difficulty range
min_diff=3
max_diff=8

# Set minimum fraction (as a power of two denominator)
min_size_denom=4  # This corresponds to 1/4

# Write header to results.txt
echo "file_size,chunks,nfisch,bit_difficulty,mvalue,init_time,proving_time,fraction,secret_key" > results.txt

# Get the full size of the original file
if [[ "$OSTYPE" == "darwin"* ]]; then
    full_size=$(stat -f%z "$data_file")
else
    full_size=$(stat -c%s "$data_file" 2>/dev/null || wc -c < "$data_file" | tr -d ' ')
fi
echo "Original $data_file size: $full_size bytes"

# Calculate the number of fractions based on min_size_denom
exponent=0
max_exponent=0
while [ $((2**max_exponent)) -lt $min_size_denom ]; do
    max_exponent=$((max_exponent + 1))
done

# Create fractions array
fractions=()
sizes=()
for ((exp=0; exp<=max_exponent; exp++)); do
    denominator=$((2**exp))
    fraction="1/$denominator"
    fraction_size=$((full_size / denominator))
    
    # Verify size is valid and power of two
    if [ $fraction_size -lt 32 ]; then
        echo "Fraction size $fraction_size too small, skipping"
        continue
    fi
    
    if ! [ $((fraction_size & (fraction_size - 1))) -eq 0 ]; then
        echo "Fraction size $fraction_size is not power of two, skipping"
        continue
    fi
    
    fractions+=("$fraction")
    sizes+=("$fraction_size")
done

# Function to parse time from Rust output
parse_time() {
    local output="$1"
    local pattern="$2"
    echo "$output" | grep "$pattern" | awk '{print $3}' | sed -E 's/([0-9]+\.[0-9]{6}).*s/\1/'
}

# Create fractional files and run prover
for i in "${!fractions[@]}"; do
    fraction=${fractions[i]}
    fraction_size=${sizes[i]}
    
    # Create filename for this fraction
    fraction_file="data_${fraction_size}.bin"
    
    # Create fractional file
    echo "Creating $fraction_file ($fraction_size bytes) representing $fraction of original..."
    
    # Use head or dd to create the file
    if command -v head &> /dev/null; then
        head -c "$fraction_size" "$data_file" > "$fraction_file"
    else
        dd if="$data_file" of="$fraction_file" bs=1 count="$fraction_size" status=none
    fi
    
    # Run prover for each bit difficulty in range
    for ((bit_difficulty=min_diff; bit_difficulty<=max_diff; bit_difficulty++)); do
        echo "Running with $fraction fraction (${fraction_size} bytes), bit_difficulty=$bit_difficulty"
        
        # Run the prover with the hardcoded secret key
        # Temporarily disable error exit for this command
        set +e
        output=$(cargo run --bin prover -- "$fraction_file" --bit-difficulty "$bit_difficulty" --secret-key "$secret_key" 2>&1)
        status=$?
        set -e
        
        if [ $status -ne 0 ]; then
            echo "ERROR: Prover failed with exit status $status"
            echo "Prover output:"
            echo "$output"
            
            # Use known values for the result row
            file_size=$fraction_size
            chunks=$((fraction_size / 32))
            nfisch=10
            mvalue=16
            init_time="ERROR"
            proving_time="ERROR"
        else
            # Parse parameters from output
            file_size=$(echo "$output" | grep "File size:" | awk '{print $3}')
            chunks=$(echo "$output" | grep "Num chunks:" | awk '{print $3}')
            nfisch=$(echo "$output" | grep "nfisch:" | awk -F '[,:]' '{print $2}' | tr -d ' ')
            mvalue=$(echo "$output" | grep "m:" | awk -F '[,:]' '{print $4}' | tr -d ' ')
            
            # Parse times from output
            init_time=$(parse_time "$output" "Initialization time:")
            proving_time=$(parse_time "$output" "Proving time:")
        fi
        
        # Format results as CSV and append to file
        echo "$file_size,$chunks,$nfisch,$bit_difficulty,$mvalue,$init_time,$proving_time,$fraction,\"$secret_key\"" >> results.txt
        
        echo "---------------------------------------------"
    done
    
    # Clean up fractional file
    rm "$fraction_file"
done

echo "All tests completed. Results saved to results.txt"
echo "CSV format: file_size,chunks,nfisch,bit_difficulty,mvalue,init_time,proving_time,fraction,secret_key"
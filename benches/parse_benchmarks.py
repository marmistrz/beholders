#!/usr/bin/env python
"""
Benchmark Results Parser

This script reads benchmark result files from the res/ directory and creates
a pandas DataFrame with file size, initialization time, FK20 time, and proving time.
"""

import os
import re
import pandas as pd
from pathlib import Path
from typing import Optional, Dict, Any, Union, cast
from humanfriendly import parse_size


def parse_file_size(size_str: str) -> Optional[int]:
    """
    Parse file size string like '128 KiB' or '1 MiB' and return size in KiB as integer.
    Uses humanfriendly.parse_size for robust parsing.
    """
    try:
        # parse_size returns bytes, convert to KiB
        size_bytes = parse_size(size_str)
        return int(size_bytes // 1024)
    except Exception:
        return None


def parse_time(time_str: str) -> Optional[float]:
    """
    Parse time string like '7.212863475s' and return time in seconds as float.
    """
    match = re.search(r'(\d+\.?\d*)s', time_str)
    if match:
        return float(match.group(1))
    return None


def parse_benchmark_file(filepath: Union[str, Path]) -> Optional[Dict[str, Union[int, float]]]:
    """
    Parse a single benchmark file and extract relevant metrics.

    Returns:
        dict: Dictionary with parsed data or None if parsing fails or any required field is missing
    """
    with open(filepath, 'r') as f:
        content = f.read()

    # Extract file size
    file_size_match = re.search(r'File size:\s*(.+)', content)
    file_size = None
    if file_size_match:
        file_size = parse_file_size(file_size_match.group(1))
    if not file_size:
        return None

    # Extract initialization time
    init_time_match = re.search(r'Initialization time:\s*(.+)', content)
    init_time = None
    if init_time_match:
        init_time = parse_time(init_time_match.group(1))
    if not init_time:
        return None

    # Extract FK20 time
    fk20_time_match = re.search(r'FK20 time:\s*(.+)', content)
    fk20_time = None
    if fk20_time_match:
        fk20_time = parse_time(fk20_time_match.group(1))
    if not fk20_time:
        return None

    # Extract proving time
    proving_time_match = re.search(r'Proving time:\s*(.+)', content)
    proving_time = None
    if proving_time_match:
        proving_time = parse_time(proving_time_match.group(1))
    if not proving_time:
        return None
    mining_time = proving_time - fk20_time

    return {
        'file_size_kib': file_size,
        'initialization_time_s': init_time,
        'fk20_time_s': fk20_time,
        'mining_time_s': mining_time,
    }


def main() -> Optional[pd.DataFrame]:
    """
    Main function to process all files in res/ directory and create DataFrame.
    """
    # Get the directory where the script is located
    script_dir = Path(__file__).parent
    res_dir = script_dir / 'res'

    if not res_dir.exists():
        print(f"Error: {res_dir} directory not found!")
        return None

    # Find all text files in res directory
    benchmark_files = list(res_dir.glob('*.txt'))

    if not benchmark_files:
        print(f"No .txt files found in {res_dir}")
        return None

    print(f"Found {len(benchmark_files)} benchmark files:")
    for f in benchmark_files:
        print(f"  - {f.name}")

    # Parse all files
    data = []
    for filepath in benchmark_files:
        parsed_data = parse_benchmark_file(filepath)
        if parsed_data and parsed_data['file_size_kib'] is not None:
            data.append(parsed_data)
        else:
            print(f"Warning: Could not parse {filepath.name}")

    if not data:
        print("No valid data found!")
        return None

    # Create DataFrame
    df = pd.DataFrame(data)

    # Sort by file size for better readability
    df = df.sort_values('file_size_kib')

    # Set file size as index for the final table
    df_final = df.set_index('file_size_kib')
    df_final.index.name = 'File Size (KiB)'
    df_final.columns = ['Initialization Time (s)', 'FK20 Time (s)', 'Mining Time (s)']

    print("\n" + "="*60)
    print("BENCHMARK RESULTS SUMMARY")
    print("="*60)
    print(df_final.to_string())

    # Save to CSV
    output_file = script_dir / 'benchmark_results.tex'
    df_final.to_latex(output_file)
    print(f"\nResults saved -to: {output_file}")

    return df_final


if __name__ == "__main__":
    df = main()
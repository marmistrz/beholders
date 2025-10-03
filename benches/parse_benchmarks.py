#!/usr/bin/env python
"""
Benchmark Results Parser

This script reads benchmark result files from the res/ directory and creates
a pandas DataFrame with file size, initialization time, FK20 time, and proving time.
"""

import re
import pandas as pd
from pathlib import Path
from typing import Optional, Dict, Union
from humanfriendly import parse_size, parse_timespan, format_timespan


def parse_file_size(size_str: str) -> Optional[int]:
    """
    Parse file size string like '128 KiB' or '1 MiB' and return size in KiB as integer.
    """
    try:
        # parse_size returns bytes, convert to KiB
        size_bytes = parse_size(size_str)
        return int(size_bytes // 1024)
    except Exception:
        return None


def parse_time(time_str: str) -> Optional[float]:
    """
    Parse time string and return time in seconds as float.
    """
    try:
        return float(parse_timespan(time_str))
    except Exception:
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

    # Group by file size and calculate averages and counts
    grouped = df.groupby('file_size_kib')

    # Calculate averages for timing metrics (excluding initialization time)
    df_averaged = grouped[['fk20_time_s', 'mining_time_s']].mean()

    # Add freshness period column (in seconds)
    df_averaged['freshness'] = df_averaged['mining_time_s'] * 2 * 1e5

    # Custom compact formatting for freshness period: e.g. 5h20min
    def compact_timespan(seconds):
        units = [
            ('d', 86400),
            ('h', 3600),
            ('min', 60),
            ('s', 1),
        ]
        remaining = int(seconds)
        result = []
        for name, unit_seconds in units:
            value, remaining = divmod(remaining, unit_seconds)
            if value > 0:
                result.append(f"{value}{name} ")
            if len(result) == 2:
                break
        return ''.join(result) if result else '0s'

    df_averaged['Freshness period'] = df_averaged['freshness'].apply(compact_timespan)

    # Set proper column names
    df_averaged.index.name = 'File Size [KiB]'
    df_averaged = df_averaged[['fk20_time_s', 'mining_time_s', 'Freshness period']]
    df_averaged.columns = ['FK20 Time [s]', 'Mining Time [s]', 'Freshness period']

    print("\n" + "="*60)
    print("BENCHMARK RESULTS SUMMARY (AVERAGED)")
    print("="*60)
    print(df_averaged.to_string())

    # Get sample counts for caption and validation
    sample_counts = grouped.size()

    # Check if all file sizes have the same number of samples
    unique_counts = sample_counts.unique()
    if len(unique_counts) == 1:
        sample_count = unique_counts[0]
        print(f"\nAll file sizes have {sample_count} samples each.")
    else:
        print(f"\nError: Sample counts differ across file sizes:")
        for file_size, count in sample_counts.items():
            print(f"  {file_size} KiB: {count} samples")
        print("Error: all file sizes must have the same number of samples.")
        return None

    df_final = df_averaged

    # Create caption with sample information
    caption = f"Benchmark results (averaged over {sample_count} samples per file size). Freshness period = Mining Time [s] × 10⁵."

    # Save to LaTeX
    ncols = df_final.shape[1] + 1  # +1 for index column
    col_format = "|".join(["c"] * ncols)
    output_file = script_dir / 'benchmark_results.tex'
    df_final.to_latex(output_file, column_format=col_format)

    # Add a comment at the top of the LaTeX file
    with open(output_file, 'r+') as f:
        content = f.read()
        f.seek(0, 0)
        f.write('% Automatically generated by parse_benchmarks.py\n% DO NOT EDIT MANUALLY\n\n' + content)
    print(f"\nAveraged results saved to: {output_file}")

    return df_final


if __name__ == "__main__":
    df = main()
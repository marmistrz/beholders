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
import argparse

FILE_SIZE_COLUMN = "File Size [KiB]"


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


def parse_prover_benchmark_file(
    filepath: Union[str, Path],
) -> Optional[Dict[str, Union[int, float]]]:
    """
    Parse a single benchmark file and extract relevant metrics.

    Returns:
        dict: Dictionary with parsed data or None if parsing fails or any required field is missing
    """
    with open(filepath, "r") as f:
        content = f.read()

    # Extract file size
    file_size_match = re.search(r"File size:\s*(.+)", content)
    file_size = None
    if file_size_match:
        file_size = parse_file_size(file_size_match.group(1))
    if not file_size:
        return None

    # Extract FK20 time
    fk20_time_match = re.search(r"FK20 time:\s*(.+)", content)
    fk20_time = None
    if fk20_time_match:
        fk20_time = parse_time(fk20_time_match.group(1))
    if not fk20_time:
        return None

    # Extract proving time
    proving_time_match = re.search(r"Proving time:\s*(.+)", content)
    proving_time = None
    if proving_time_match:
        proving_time = parse_time(proving_time_match.group(1))
    if not proving_time:
        return None
    mining_time = proving_time - fk20_time

    return {
        FILE_SIZE_COLUMN: file_size,
        "FK20 Time [s]": fk20_time,
        "Mining Time [s]": mining_time,
    }


def parse_verifier_benchmark_file(
    filepath: Union[str, Path],
) -> Optional[Dict[str, float]]:
    """
    Parse a single verifier benchmark file and extract relevant metrics.

    Returns:
        dict: Dictionary with parsed data or None if parsing fails or any required field is missing
    """
    with open(filepath, "r") as f:
        content = f.read()

    # Extract file size from filename, e.g., out512-3.txt -> 512
    filename = Path(filepath).name
    file_size_match = re.match(r"out(\d+)-", filename)
    if not file_size_match:
        return None
    try:
        file_size_kib = int(file_size_match.group(1))
    except Exception:
        return None

    # Extract verification time (e.g., 'Verification took: 11.865ms')
    verification_time_match = re.search(r"Verification took:\s*([\d.]+)ms", content)
    if not verification_time_match:
        return None
    try:
        verification_time_ms = float(verification_time_match.group(1))
    except Exception:
        return None

    return {
        FILE_SIZE_COLUMN: file_size_kib,
        "Verification Time [ms]": verification_time_ms,
    }


# Custom compact formatting for freshness period: e.g. 5h20min
def compact_timespan(seconds):
    units = [
        ("d", 86400),
        ("h", 3600),
        ("min", 60),
        ("s", 1),
    ]
    remaining = int(seconds)
    result = []
    for name, unit_seconds in units:
        value, remaining = divmod(remaining, unit_seconds)
        if value > 0:
            result.append(f"{value}{name} ")
        if len(result) == 2:
            break
    return "".join(result) if result else "0s"


def main() -> None:
    """
    Main function to process prover or verifier benchmarks based on argument.
    """
    parser = argparse.ArgumentParser(description="Benchmark Results Parser")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument(
        "--prover", action="store_true", help="Parse prover benchmarks (from res/)"
    )
    group.add_argument(
        "--verifier",
        action="store_true",
        help="Parse verifier benchmarks (from verifier/)",
    )
    args = parser.parse_args()

    script_dir = Path(__file__).parent
    if args.prover:
        bench_dir = script_dir / "prover"
    else:
        bench_dir = script_dir / "verifier"

    if not bench_dir.exists():
        print(f"Error: {bench_dir} directory not found!")
        return None

    benchmark_files = list(bench_dir.glob("*.txt"))
    if not benchmark_files:
        print(f"No .txt files found in {bench_dir}")
        return None

    print(f"Found {len(benchmark_files)} benchmark files:")

    # Parse all files
    data = []
    for filepath in benchmark_files:
        parsed_data = (
            parse_prover_benchmark_file(filepath)
            if args.prover
            else parse_verifier_benchmark_file(filepath)
        )
        if parsed_data and parsed_data[FILE_SIZE_COLUMN] is not None:
            data.append(parsed_data)
        else:
            print(f"Warning: Could not parse {filepath.name}")

    if not data:
        print("No valid data found!")
        return None

    # Create DataFrame
    df = pd.DataFrame(data)

    # Group by file size and calculate averages and counts
    grouped = df.groupby(FILE_SIZE_COLUMN)

    # Calculate averages for timing metrics (excluding initialization time)
    columns = [col for col in df.columns if col != FILE_SIZE_COLUMN]
    df_averaged = grouped[columns].mean()

    if args.prover:
        # Add freshness period column directly (formatted)
        df_averaged["Freshness period"] = (
            df_averaged["Mining Time [s]"] * 2 * 1e5
        ).apply(compact_timespan)
        columns.append("Freshness period")

    print("\n" + "=" * 60)
    print("BENCHMARK RESULTS SUMMARY (AVERAGED)")
    print("=" * 60)
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
    # caption = f"Benchmark results (averaged over {sample_count} samples per file size). Freshness period = Mining Time [s] × 10⁵."

    # Save to LaTeX
    ncols = df_final.shape[1] + 1  # +1 for index column
    col_format = "|".join(["c"] * ncols)
    if args.prover:
        output_file = script_dir / "benchmark_results_prover.tex"
    else:
        output_file = script_dir / "benchmark_results_verifier.tex"
    df_final.to_latex(output_file, column_format=col_format)

    # Add a comment at the top of the LaTeX file
    with open(output_file, "r+") as f:
        content = f.read()
        f.seek(0, 0)
        f.write(
            "% Automatically generated by parse_benchmarks.py\n% DO NOT EDIT MANUALLY\n\n"
            + content
        )
    print(f"\nAveraged results saved to: {output_file}")


if __name__ == "__main__":
    main()

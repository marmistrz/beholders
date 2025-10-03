# Benchmarks
## Integration benchmarks
First, generate the benchmark files:
```
./gen.sh
```
then run the benchmarks as
```
./bench.sh --num-iterations N
```
The execution logs will be written to the `res` subdirectory. The logs contain:
- initialization time (reading the trusted setup and building KZG); building KZG from the trusted setup is actually quite
- FK20 opening computation; this step could be reused between multiple proofs sharing the same data
- total proving time (excluding initialization, including FK20 computation); the exe

For full options of `bench.sh`, run `bench.sh --help`.

## Microbenchmarks
Microbenchmarks are implemented using criterion and can be invoked using
```
cargo bench
```

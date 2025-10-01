# Benchmarks
## Integration benchmarks
First, generate the benchmark files:
```
./gen.sh
```
then run the benchmarks as
```
./bench.sh
```
The execution logs will be written to the `res` subdirectory. The logs contain:
- initialization time (reading the trusted setup)
- FK20 opening computation
- total proving time (excluding initialization, including FK20 computation)

## Microbenchmarks
Microbenchmarks are implemented using criterion and can be invoked using
```
cargo bench
```

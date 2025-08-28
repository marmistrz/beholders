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
The execution logs will be written to the `res` subdirectory.

## Microbenchmarks
Microbenchmarks are implemented using criterion and can be invoked using
```
cargo bench
```

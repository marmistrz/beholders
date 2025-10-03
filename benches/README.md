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
- initialization time: this involves loading the trusted setup and building KZG out of it; this step could, in the future, be eliminated by serializing the whole KZG settings (including precomputation tables) and not just the trusted setup;
- FK20 opening computation; this step could be trustlessly reused between multiple instances sharing the same data;
- total proving time (FK20 and mining); mining is the only step that _cannot_ be reused between multiple instances.

For full options of `bench.sh`, run `bench.sh --help`.

## Microbenchmarks
Microbenchmarks are implemented using criterion and can be invoked using
```
cargo bench
```

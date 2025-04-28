# Beholders
## Usage guide
To simplify the process, the proof-of-concept uses a hardcoded keypair.
For real-life use cases, these should be generated separately and read from a file.

### Generating trusted setup
The trusted setup established by the Ethereum community in their KZG ceremony can only
support beholder signatures for files up to 4096 chunks, i.e., 131072 bytes. Since we aim to support
bigger files, the setup should be generated as
```
cargo run --bin setup --release -- --secrets N secrets.bin
```
Where \(N = l/32 \) where \(l\) is the maximum size of the file in bytes.

### Run prover
In order to generate a beholder signature on the file `data.bin`, execute the following command:
```
cargo run --bin prover --release -- --bit-difficulty 10 --setup-file secrets.bin data.bin com.bin sig.bin
```
In this case, the setup is read from `secrets.bin`, the signature is saved as `sig.bin`,
and the commitment is written to `com.bin`.
### Run verifier
In order to verify the signature generated as above, run:
```
cargo run --bin verifier --release -- --bit-difficulty 10 --setup-file secrets.bin --data-len $(du -b data128.bin) com.bin sig.bin
```
## Developer guide
### Run tests
```
cargo nextest run
```
### Run benchmarks
```
cargo bench
```
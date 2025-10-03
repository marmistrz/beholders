# Beholders
## Usage guide
Since the beacon value does not influence the performance of the implementation, we omitted it for simplicity.

### Generating trusted setup
The trusted setup established by the Ethereum community in their KZG ceremony can only
support beholder signatures for files up to 4096 chunks, i.e., 131072 bytes. Since we aim to support
bigger files, the setup should be generated as:
```
cargo run --bin setup --release -- --secrets N secrets.bin
```
Where $N = \ell/32$ where $\ell$ is the maximum size of the file in bytes.

### Run keygen
To generate a keypair, run
```
cargo run --bin kgen --release -- --secret-key=sk.bin --public-key=pk.bin
```

### Run prover
In order to generate a beholder signature on the file `data.bin`, execute the following command:
```
cargo run --bin prover --release -- --secret-key sk.bin --setup-file secrets.bin data.bin com.bin sig.bin
```
In this case, the setup is read from `secrets.bin`, the signature is saved as `sig.bin`,
and the commitment is written to `com.bin`.
### Run verifier
In order to verify the signature generated as above, run:
```
cargo run --bin verifier --release -- --public-key pk.bin --setup-file secrets.bin --data-len $(du -b data128.bin) com.bin sig.bin
```
## Developer guide
### Run tests
```
cargo nextest run
```
## Run benchmarks
See [benches/README.md](benches/README.md)

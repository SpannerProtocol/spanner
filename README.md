# Spanner

Install required compiler version and wasm component

```bash
rustup target add wasm32-unknown-unknown --toolchain nightly
rustup default nightly-x86_64-apple-darwin

# compile
cargo build --release

# run test
cargo test
cargo test -- --ignored
# run benchmark test
cargo test -p pallet-balances --features runtime-benchmarks

# run a temporary local node (no data is stored)
target/release/substrate --dev --tmp

# purge chain data
cargo run -- --purch-chain --dev

# to view documentation
cd spanner/pallets/bullet-train
cargo doc --open``

# generate weights
# https://crates.io/crates/frame-benchmarking
cd spanner/cli
cargo build --release --features runtime-benchmarks

cd spanner
./target/release/substrate benchmark --pallet "*" --extrinsic "*" --repeat 100
./target/release/substrate benchmark --pallet pallet_bullet-train --extrinsic "*" --repeat 100 --output pallet/poc/src

./target/release/substrate \
   benchmark \
   --chain=dev \
   --steps=50 \
   --repeat=20 \
   --pallet="*" \
   --extrinsic="*" \
   --execution=wasm \
   --wasm-execution=compiled \
   --heap-pages=4096 \
   --output=./runtime/src/weights
   
./target/release/substrate \
  benchmark \
  --chain=dev \
  --steps=50 \
  --repeat=20 \
  --pallet=pallet_dex \
  --extrinsic="*" \
  --execution=wasm \
  --wasm-execution=compiled \
  --heap-pages=4096 \
  --output=./pallets/dex/src/weights.rs \
  --template=./template.hbs
  
  
./target/release/substrate \
  benchmark \
  --chain=dev \
  --steps=50 \
  --repeat=20 \
  --pallet=pallet_bullet_train \
  --extrinsic="*" \
  --execution=wasm \
  --wasm-execution=compiled \
  --heap-pages=4096 \
  --output=./pallets/bullet-train/src/weights.rs \
  --template=./template.hbs

# To export chain spec into .json file
./target/release/substrate build-spec --chain local --disable-default-bootnode > spec/local.json

# Produce WASM for forkless update
cargo build --release -p node-runtime
```

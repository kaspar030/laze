#!/bin/sh
set -e

# STEP 0: Make sure there is no left-over profiling data from previous runs
rm -rf /tmp/pgo-data

# STEP 1: Build the instrumented binaries
RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data -Clink-arg=-Wl,--emit-relocs" \
    cargo build --release --target=x86_64-unknown-linux-gnu

touch RIOT/laze-project.yml
./target/x86_64-unknown-linux-gnu/release/laze -C RIOT b -g -G

/home/kaspar/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-profdata merge -o /tmp/pgo-data/merged.profdata /tmp/pgo-data

RUSTFLAGS="-Cprofile-use=/tmp/pgo-data/merged.profdata -Clink-arg=-Wl,--emit-relocs" \
    cargo build --release --target=x86_64-unknown-linux-gnu

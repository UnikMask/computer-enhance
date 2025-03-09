#!/bin/env sh

cargo build --release

for f in "simple" "complex"; do
    nasm examples/"$f".asm
    target/release/decode_8086 examples/"$f"
    vimdiff examples/"$f".asm results/"$f".asm
done

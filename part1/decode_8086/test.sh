#!/bin/env sh

cargo build --release

for f in "simple" "complex"; do
    nasm examples/"$f".asm
    target/release/decode_8086 examples/"$f" > results/"$f".asm
    nasm results/"$f".asm
    diff examples/"$f" results/"$f"
done

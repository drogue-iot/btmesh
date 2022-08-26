#!/bin/bash
echo "NOTE: This assumes you've installd cargo call-stack: cargo install --git https://github.com/lulf/cargo-call-stack.git --branch weird-output"
cargo size --release | tail -n 2 > memusage.txt
echo "Stack" >> memusage.txt
cargo call-stack --bin basic --format top 2> callstack.error | head -n 20 >> memusage.txt

max=$(head -4 memusage.txt | tail -1 | cut -f 1 -w)
bss=$(head -2 memusage.txt  | tail -1  | cut -f 4 -w)

ram=$(dc -e "$bss $max + 1024 / p")

echo "======> $ram KB <======"

cat memusage.txt

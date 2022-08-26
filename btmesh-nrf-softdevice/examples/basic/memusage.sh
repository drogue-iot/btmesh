#!/bin/bash
echo "NOTE: This assumes you've installd cargo call-stack: cargo install --git https://github.com/lulf/cargo-call-stack.git --branch weird-output"
cargo size --release | tail -n 2 > memusage.txt
echo "Stack" >> memusage.txt
cargo call-stack --bin basic --format top 2> callstack.error | head -n 10 >> memusage.txt

echo "Done, results are stored in memusage.txt"

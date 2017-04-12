#!/bin/sh

# "Compile rust source and put it as a tested contract"

mkdir -p out
rustc $1 -o out/contract.js -O --target wasm32-unknown-emscripten
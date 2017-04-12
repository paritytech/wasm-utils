#!/bin/sh

# "Compile rust source and put it as a tested contract"

mkdir -p out

file=$1
if [ ${file: -3} == ".rs" ]
then
    # Rust is compiled with rustc
    rustc $file -o out/contract.js -O --target wasm32-unknown-emscripten
else
    # c/c++ can be compiled directly by emcc
    emcc $file -Os -s WASM=1 -s SIDE_MODULE=1 -o out/contract.wasm
fi

if [ ! -f ./../gas/target/release/gas ] && [ ! -f ./../gas/target/release/gas.exe ] 
then
    echo "No gas utility, compile it in /gas folder with"
    echo "cargo build --release"
else
    ./../gas/target/release/gas ./out/contract.wasm ./out/contract.wasm
fi

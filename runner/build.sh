#!/bin/sh

# "Compile rust source and put it as a tested contract"

mkdir -p out

file=$1
if [ ${file: -3} == ".rs" ]
then
    # Rust is compiled with rustc
    rustc $file -o out/contract.js -O --target wasm32-unknown-emscripten

    # Gas injector
    cargo run --manifest-path=./../gas/Cargo.toml --release -- ./out/contract.wasm ./out/contract.wasm

    # Allocator replacer
    cargo run --manifest-path=./../ext/Cargo.toml --release -- ./out/contract.wasm ./out/contract.wasm

    # Symbols optimizer
    cargo run --manifest-path=./../opt/Cargo.toml --release -- ./out/contract.wasm ./out/contract.wasm

else
    # c/c++ can be compiled directly by emcc
    emcc $file -O3 -s WASM=1 -s SIDE_MODULE=1 -o out/contract.wasm

    # Gas injector
    cargo run --manifest-path=./../gas/Cargo.toml --release -- ./out/contract.wasm ./out/contract.wasm

    # Symbols optimizer
    cargo run --manifest-path=./../opt/Cargo.toml --release -- ./out/contract.wasm ./out/contract.wasm
fi



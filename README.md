# wasm-tools

Boilerplate code to test Parity WASM tools and compile rust/c/c++ code to wasm-contracts

## How to compile contract

```
git clone https://github.com/NikVolf/wasm-tools
cd wasm-tools/runner
./build.sh <PATH TO C/C++/Rust source file>
./start.sh
```

and then open `http://localhost:8000`, press `Execute call` to run a contract `call` function, see browser console log for gas counter

see `/samples` directory for sample contracts that are compiled this way

## Prerequisites 

- Emscripiten for C/C++ (see [this page](http://kripken.github.io/emscripten-site/docs/getting_started/downloads.html), `emcc` should be in the `PATH`)
- Rust with `wasm32-unknown-emscripten` target (see [this instruction](https://hackernoon.com/compiling-rust-to-webassembly-guide-411066a69fde) to setup)

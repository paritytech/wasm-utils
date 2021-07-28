# pwasm-utils

[![Build Status](https://travis-ci.org/paritytech/wasm-utils.svg?branch=master)](https://travis-ci.org/paritytech/wasm-utils)

A collection of WASM utilities used in pwasm-ethereum and substrate contract development.

This repository contains the package `pwasm-utils` which consists of a library crate
and a collection of cli binaries that make use of this library.

## Installation of cli tools
```
cargo install pwasm-utils --features cli
```

This will install the following binaries:
* wasm-build
* wasm-check
* wasm-ext
* wasm-gas
* wasm-pack
* wasm-prune
* wasm-stack-height

## Symbols pruning (wasm-prune)

```
wasm-prune <input_wasm_binary.wasm> <output_wasm_binary.wasm>
```

This will optimize WASM symbols tree to leave only those elements that are used by contract `call` function entry.

## Gas counter (wasm-gas)

For development purposes, a raw WASM contract can be injected with gas counters (the same way as it done in the `pwasm-ethereum/substrate` runtime when running contracts)

```
wasm-gas <input_wasm_binary.wasm> <output_wasm_binary.wasm>
```

# License

`wasm-utils` is primarily distributed under the terms of both the MIT
license and the Apache License (Version 2.0), at your choice.

See LICENSE-APACHE, and LICENSE-MIT for details.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in `wasm-utils` by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

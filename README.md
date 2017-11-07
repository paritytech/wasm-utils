# wasm-utils

Collection of WASM utilities used in Parity and WASM contract devepment

## Build tools for cargo

Easiest way to use is to install via `cargo install`:

```
cargo install --git https://github.com/paritytech/wasm-utils wasm-build
```

## Symbols pruning (wasm-prune)

```
cargo run --release --bin wasm-prune -- <input_binary.wasm> <output_binary.wasm>
```

This will optimize WASM symbols tree to leave only those elements that are used by contract `_call` function entry.

## Gas counter (wasm-gas)

For development puposes, raw WASM contract can be injected with gas counters (the same way as it done by Parity runtime when running contracts)

```
cargo run --release --bin wasm-gas -- <input_binary.wasm> <output_binary.wasm>
```

## Externalization (wasm-ext)

Parity WASM runtime provides some library functions that can be commonly found in libc. WASM binary size can be reduced and performance may be improved if these functions are used. This utility scans for invocations of the following functions inside the WASM binary:
- `_malloc`,
- `_free`,
- `_memcpy`,
- `_memset`,
- `_memmove`

And then substitutes them with invocations of the imported ones. Should be run before `wasm-opt` for better results.

```
cargo run --release --bin wasm-ext -- <input_binary.wasm> <output_binary.wasm>
```

## API

All executables use corresponding api methods of the root crate and can be combined in other build tools.

# License

`wasm-utils` is primarily distributed under the terms of both the MIT
license and the Apache License (Version 2.0), at your choice.

See LICENSE-APACHE, and LICENSE-MIT for details.

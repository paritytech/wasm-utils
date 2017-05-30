# wasm-utils

Collection of WASM utilities used in Parity and WASM contract devepment

## Symbols optimizer (wasm-opt)

```
cargo run --release --bin wasm-opt -- <input_binary.wasm> <output_binary.wasm>
```

This will optimize WASM symbols tree to leave only those elements that are used by contract `call` function entry.

## Gas counter (wasm-gas)

For development puposes, raw WASM contract can be injected with gas counters (the same way as it done by Parity runtime when running contracts)

```
cargo run --release --bin wasm-gas -- <input_binary.wasm> <output_binary.wasm>
```

## Allocators substiution (wasm-ext)

Parity WASM runtime provides simple memory allocators, if contract requires. When relied on this allocators, WASM binary size can be greatly reduced. This utility scans for `_malloc`, `_free` invokes inside the WASM binary and substitutes it with invokes of the imported `_malloc`, `_free`. Should be run before `wasm-opt` for better results.

```
cargo run --release --bin wasm-ext -- <input_binary.wasm> <output_binary.wasm>
```

## API

All executables use corresponding api methods of the root crate and can be combined in other build tools.

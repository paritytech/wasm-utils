#![feature(link_args)]
#![no_main]

// as it is experimental preamble
#![allow(dead_code)]

#[link_args = "-s WASM=1 -s NO_EXIT_RUNTIME=1 -s NO_FILESYSTEM=1 -s"]
extern {}

#[no_mangle]
pub fn call() {
}
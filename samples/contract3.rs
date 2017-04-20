#![feature(link_args)]
#![feature(drop_types_in_const)]
#![no_main]

use std::slice;

#[link_args = "-s WASM=1 -s NO_EXIT_RUNTIME=1 -s NO_FILESYSTEM=1"]
extern {}

#[no_mangle]
pub fn call(input: *mut u8) {
    let mut slice = unsafe { slice::from_raw_parts_mut(input, 8192) }; // 8kb input data
    for i in 0..slice.len() {
        slice[i] = slice[i] + 2;
    } 
}
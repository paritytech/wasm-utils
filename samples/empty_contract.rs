#![feature(link_args)]
#![no_main]

// as it is experimental preamble
#![allow(dead_code)]

#[link_args = "-s WASM=1 -s NO_EXIT_RUNTIME=1 -s NO_FILESYSTEM=1"]
extern {}

#[no_mangle]
pub fn call() {
}

/* This produces the following code (after injecting gas counter & optimizing)
(module
  (type (;0;) (func))
  (type (;1;) (func (param i32)))
  (import "env" "memory" (memory (;0;) 256 256))
  (import "env" "table" (table (;0;) 0 0 anyfunc))
  (import "env" "gas" (func (;0;) (type 1)))
  (func (;1;) (type 0)
    i32.const 2
    call 0
    nop)
  (export "_call" (func 1))
  (data (i32.const 1212) "\1c\05"))

*/
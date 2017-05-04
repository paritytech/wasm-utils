#![feature(link_args)]
#![no_main]

// as it is experimental preamble
#![allow(dead_code)]

#[link_args = "-s WASM=1 -s NO_EXIT_RUNTIME=1 -s NO_FILESYSTEM=1"]
extern {}

static DATA: u32 = 0;

#[no_mangle]
pub fn call(_descr: *mut u8) {
    let data_ptr = &DATA as *const u32 as *mut u32;
    unsafe { *data_ptr += 1; }
}

/* This produces the following code (after injecting gas counter & optimizing)
(module
  (type (;0;) (func (param i32)))
  (type (;1;) (func (param i32)))
  (import "env" "memory" (memory (;0;) 256 256))
  (import "env" "table" (table (;0;) 0 0 anyfunc))
  (import "env" "gas" (func (;0;) (type 1)))
  (func (;1;) (type 0) (param i32)
    i32.const 4
    call 0
    i32.const 1268
    i32.const 1
    i32.store)
  (export "_call" (func 1))
  (data (i32.const 1212) " \05"))
*/
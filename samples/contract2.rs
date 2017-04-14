#![feature(link_args)]
#![feature(drop_types_in_const)]
#![no_main]

#[link_args = "-s WASM=1 -s NO_EXIT_RUNTIME=1 -s NO_FILESYSTEM=1"]
extern {}

static mut DATA: Option<Vec<u8>> = None;

#[no_mangle]
pub fn call() {
    let mut vec = Vec::new();
    unsafe { if let Some(ref v) = DATA { vec.extend(v); }; }
    vec.push(1u8);
    unsafe {
        DATA = Some(vec);
    }
}
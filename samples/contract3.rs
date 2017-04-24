#![feature(link_args)]
#![feature(drop_types_in_const)]
#![no_main]

// as it is experimental preamble
#![allow(dead_code)]

use std::slice;

#[link_args = "-s WASM=1 -s NO_EXIT_RUNTIME=1 -s NO_FILESYSTEM=1 -s"]
extern {}

/// Safe (?) wrapper around call context
struct CallArgs {
    context: Box<[u8]>,
    result: Vec<u8>,
    storage: Vec<u8>,
}

unsafe fn read_ptr_mut(slc: &[u8]) -> *mut u8 {
    std::ptr::null_mut().offset(read_u32(slc) as isize)
}

fn read_u32(slc: &[u8]) -> u32 {
    use std::ops::Shl;
    (slc[0] as u32) + (slc[1] as u32).shl(8) + (slc[2] as u32).shl(16) + (slc[3] as u32).shl(24)
}

fn write_u32(dst: &mut [u8], val: u32) {
    dst[0] = (val & 0x000000ff) as u8;
    dst[1] = (val & 0x0000ff00 >> 8) as u8;
    dst[2] = (val & 0x00ff0000 >> 16) as u8;
    dst[3] = (val & 0xff000000 >> 24) as u8;
}

fn write_ptr(dst: &mut [u8], ptr: *mut u8) {
    // todo: consider: add assert that arch is 32bit
    write_u32(dst, ptr as usize as u32);
}

impl CallArgs {
    pub fn from_raw(ptr: *mut u8) -> CallArgs {
        let desc_slice = unsafe { slice::from_raw_parts(ptr, 6 * 4) };

        let context_ptr = unsafe { read_ptr_mut(&desc_slice[0..4]) };
        let context_len = read_u32(&desc_slice[4..8]) as usize;

        let storage_ptr = unsafe { read_ptr_mut(&desc_slice[8..12]) };
        let storage_len = read_u32(&desc_slice[12..16]) as usize;

        let result_ptr = unsafe { read_ptr_mut(&desc_slice[16..20]) };
        let result_len = read_u32(&desc_slice[20..24]) as usize;

        CallArgs {
            context: unsafe { Box::<[u8]>::from_raw(slice::from_raw_parts_mut(context_ptr, context_len)) },
            result: unsafe { Vec::from_raw_parts(result_ptr, result_len, result_len) },
            // todo: consider: storage (and result?) might also have initial allocation size passed in
            // the descriptor along with length
            storage: unsafe { Vec::from_raw_parts(storage_ptr, storage_len, storage_len) },
        }
    }

    pub fn context(&self) -> &[u8] {
        &self.context
    }

    pub fn result_mut(&mut self) -> &mut Vec<u8> {
        &mut self.result
    }

    pub fn storage(&self) -> &[u8] {
        &self.storage
    }

    pub fn storage_mut(&mut self) -> &mut Vec<u8> {
        &mut self.storage
    }

    pub fn save(self, ptr: *mut u8) {
        let dst = unsafe { slice::from_raw_parts_mut(ptr, 6 * 4) };
        let context = self.context;
        let mut result = self.result;
        let mut storage = self.storage; 

        // context unmodified and memory is managed in calling code
        std::mem::forget(context);

        write_ptr(dst, storage.as_mut_ptr());
        write_u32(dst, storage.len() as u32);
        // managed in calling code
        std::mem::forget(storage);

        write_ptr(dst, result.as_mut_ptr());
        write_u32(dst, result.len() as u32);
        // managed in calling code
        std::mem::forget(result);
    }

}

#[no_mangle]
pub fn call(descriptor: *mut u8) {
    let context = CallArgs::from_raw(descriptor);
}
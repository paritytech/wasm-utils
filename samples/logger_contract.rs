#![feature(link_args)]
#![no_main]

// as it is experimental preamble
#![allow(dead_code)]

use std::slice;

#[link_args = "-s NO_EXIT_RUNTIME=1 -s NO_FILESYSTEM=1 -s"]
extern {}

/// Wrapper over storage read/write externs
/// Storage api is a key-value storage where both key and value are 32 bytes in len
mod storage {
    pub struct Error;

    #[link(name = "env")]
    extern {
        fn storage_read(key: *const u8, dst: *mut u8) -> i32;
        fn storage_write(key: *const u8, src: *const u8) -> i32;
    }

    /// Performs read from storage to the specified slice `dst`
    /// Can return `Error` if data is read from outside of the storage boundaries
    pub fn read(key: &[u8; 32], dst: &mut [u8; 32]) -> Result<(), Error> {
        match unsafe {
            let mut dst = dst;
            storage_read(key.as_ptr(), dst.as_mut_ptr())
        } {
            x if x < 0 => Err(Error),
            _ => Ok(()),
        }
    }

    /// Performs write to the storage from the specified `src`
    pub fn write(key: &[u8; 32], src: &[u8; 32]) -> Result<(), Error> {
        match unsafe {
            storage_write(key.as_ptr(), src.as_ptr())
        } {
            x if x < 0 => Err(Error),
            _ => Ok(()),
        }
    }
}

/// Safe wrapper for call context
struct CallArgs {
    context: Box<[u8]>,
    result: Vec<u8>,
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
    dst[1] = ((val & 0x0000ff00) >> 8) as u8;
    dst[2] = ((val & 0x00ff0000) >> 16) as u8;
    dst[3] = ((val & 0xff000000) >> 24) as u8;
}

fn write_ptr(dst: &mut [u8], ptr: *mut u8) {
    // todo: consider: add assert that arch is 32bit
    write_u32(dst, ptr as usize as u32);
}

impl CallArgs {
    pub fn from_raw(ptr: *mut u8) -> CallArgs {
        let desc_slice = unsafe { slice::from_raw_parts(ptr, 4 * 4) };

        let context_ptr = unsafe { read_ptr_mut(&desc_slice[0..4]) };
        let context_len = read_u32(&desc_slice[4..8]) as usize;

        let result_ptr = unsafe { read_ptr_mut(&desc_slice[8..12]) };
        let result_len = read_u32(&desc_slice[12..16]) as usize;

        CallArgs {
            context: unsafe { Box::<[u8]>::from_raw(slice::from_raw_parts_mut(context_ptr, context_len)) },
            result: unsafe { Vec::from_raw_parts(result_ptr, result_len, result_len) },
        }
    }

    pub fn context(&self) -> &[u8] {
        &self.context
    }

    pub fn result_mut(&mut self) -> &mut Vec<u8> {
        &mut self.result
    }

    pub fn save(self, ptr: *mut u8) {
        let dst = unsafe { slice::from_raw_parts_mut(ptr.offset(8), 2 * 4) };
        let context = self.context;
        let mut result = self.result;

        // context unmodified and memory is managed in calling code
        std::mem::forget(context);

        if result.len() > 0 {
            // result
            write_ptr(&mut dst[0..4], result.as_mut_ptr());
            write_u32(&mut dst[4..8], result.len() as u32);
            // managed in calling code
            std::mem::forget(result);
        }
    }
}

#[no_mangle]
pub fn call(descriptor: *mut u8) {
    // This initializes safe wrapper for contract input and output
    let mut ctx = CallArgs::from_raw(descriptor);

    // Copies all contract input data to the separate buffer
    let data = ctx.context().to_vec();

    let storage_key = [1u8; 32];
    let mut storage_val = [0u8; 32];
    storage_val.copy_from_slice(&data[0..32]);

    // Sets the key [1, 1, 1 ..., 1] to the first 32 bytes of passed input
    let _ = storage::write(&storage_key, &mut storage_val);

    // Returns all that passed to this contract as an output
    *ctx.result_mut() = data;

    // Saves the wrapper state to commit return stream
    ctx.save(descriptor);
}

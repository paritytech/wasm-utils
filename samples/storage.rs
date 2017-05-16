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

    /// Performs read from storage to the specified slice `dst`, using all slice length
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

    /// Performs write to the storage from the specified slice `src`
    pub fn write(key: &[u8; 32], src: &[u8; 32]) -> Result<(), Error> {
        match unsafe {
            storage_write(key.as_ptr(), src.as_ptr())
        } {
            x if x < 0 => Err(Error),
            _ => Ok(()),
        }
    }
}

#[no_mangle]
pub fn call(_descriptor: *mut u8) {
    let storage_key = [1u8; 32];
    let mut storage_val = [2u8; 32];
    let storage_dup_key = [3u8; 32];

    let _ = storage::write(&storage_key, &storage_val);
    let _ = storage::read(&storage_dup_key, &mut storage_val);
    let _ = storage::write(&storage_key, &storage_val);
}

//! Contains the PPMd C-code from the 7-Zip version 24.09 release.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

pub mod native;

pub const PPMD7_MIN_ORDER: u32 = 2;
pub const PPMD7_MAX_ORDER: u32 = 64;
pub const PPMD7_MIN_MEM_SIZE: u32 = 2048;
pub const PPMD7_MAX_MEM_SIZE: u32 = 4294967259;
pub const PPMD7_SYM_END: i32 = -1;
pub const PPMD7_SYM_ERROR: i32 = -2;
pub const PPMD8_MIN_ORDER: u32 = 2;
pub const PPMD8_MAX_ORDER: u32 = 16;
pub const PPMD8_SYM_END: i32 = -1;
pub const PPMD8_SYM_ERROR: i32 = -2;

pub type size_t = usize;
pub type Byte = u8;
pub type UInt16 = u16;
pub type Int32 = i32;
pub type UInt32 = u32;
pub type __uint64_t = u64;
pub type uint64_t = u64;
pub type UInt64 = u64;
pub type BoolInt = i32;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct IByteIn_ {
    pub Read: Option<unsafe extern "C" fn(IByteInPtr) -> Byte>,
}

pub type IByteInPtr = *const IByteIn_;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct IByteOut_ {
    pub Write: Option<unsafe extern "C" fn(IByteOutPtr, Byte)>,
}
pub type IByteOutPtr = *const IByteOut_;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ISzAlloc {
    pub Alloc: Option<unsafe extern "C" fn(ISzAllocPtr, size_t) -> *mut std::ffi::c_void>,
    pub Free: Option<unsafe extern "C" fn(ISzAllocPtr, *mut std::ffi::c_void)>,
}

pub type ISzAllocPtr = *const ISzAlloc;

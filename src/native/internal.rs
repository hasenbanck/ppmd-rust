pub mod ppmd7;
pub mod ppmd7dec;
pub mod ppmd7enc;
pub mod ppmd8;
pub mod ppmd8dec;
pub mod ppmd8enc;

pub type size_t = usize;
pub type Byte = std::ffi::c_uchar;
pub type UInt16 = std::ffi::c_ushort;
pub type Int32 = std::ffi::c_int;
pub type UInt32 = std::ffi::c_uint;
pub type __uint64_t = u64;
pub type uint64_t = u64;
pub type UInt64 = u64;
pub type BoolInt = std::ffi::c_int;

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
    pub Alloc: Option<unsafe extern "C" fn(ISzAllocPtr, usize) -> *mut std::ffi::c_void>,
    pub Free: Option<unsafe extern "C" fn(ISzAllocPtr, *mut std::ffi::c_void)>,
}

pub type ISzAllocPtr = *const ISzAlloc;

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct CPpmd_See {
    pub Summ: UInt16,
    pub Shift: Byte,
    pub Count: Byte,
}

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct CPpmd_State {
    pub Symbol: Byte,
    pub Freq: Byte,
    pub Successor_0: UInt16,
    pub Successor_1: UInt16,
}

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct CPpmd_State2_ {
    pub Symbol: Byte,
    pub Freq: Byte,
}

pub type CPpmd_State2 = CPpmd_State2_;

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct CPpmd_State4_ {
    pub Successor_0: UInt16,
    pub Successor_1: UInt16,
}

pub type CPpmd_State4 = CPpmd_State4_;
pub type CPpmd_State_Ref = UInt32;
pub type CPpmd_Void_Ref = UInt32;
pub type CPpmd_Byte_Ref = UInt32;

#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed {
    pub Stats: CPpmd_State_Ref,
    pub State4: CPpmd_State4,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_0 {
    pub SummFreq: UInt16,
    pub State2: CPpmd_State2,
}

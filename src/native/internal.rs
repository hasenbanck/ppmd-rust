pub mod ppmd7;
pub mod ppmd7dec;
pub mod ppmd7enc;
pub mod ppmd8;
pub mod ppmd8dec;
pub mod ppmd8enc;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct IByteIn_ {
    pub Read: Option<unsafe extern "C" fn(IByteInPtr) -> u8>,
}

pub type IByteInPtr = *const IByteIn_;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct IByteOut_ {
    pub Write: Option<unsafe extern "C" fn(IByteOutPtr, u8)>,
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
    pub Summ: u16,
    pub Shift: u8,
    pub Count: u8,
}

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct CPpmd_State {
    pub Symbol: u8,
    pub Freq: u8,
    pub Successor_0: u16,
    pub Successor_1: u16,
}

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct CPpmd_State2_ {
    pub Symbol: u8,
    pub Freq: u8,
}

pub type CPpmd_State2 = CPpmd_State2_;

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct CPpmd_State4_ {
    pub Successor_0: u16,
    pub Successor_1: u16,
}

pub type CPpmd_State4 = CPpmd_State4_;
pub type CPpmd_State_Ref = u32;
pub type CPpmd_Void_Ref = u32;
pub type CPpmd_Byte_Ref = u32;

#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed {
    pub Stats: CPpmd_State_Ref,
    pub State4: CPpmd_State4,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_0 {
    pub SummFreq: u16,
    pub State2: CPpmd_State2,
}

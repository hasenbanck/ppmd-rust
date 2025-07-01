pub(crate) mod ppmd7;

pub(crate) mod ppmd8;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct IByteIn_ {
    pub read: Option<fn(IByteInPtr) -> u8>,
}

pub type IByteInPtr = *const IByteIn_;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct IByteOut_ {
    pub write: Option<fn(IByteOutPtr, u8)>,
}
pub type IByteOutPtr = *const IByteOut_;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ISzAlloc {
    pub alloc: Option<fn(ISzAllocPtr, usize) -> *mut std::ffi::c_void>,
    pub free: Option<fn(ISzAllocPtr, *mut std::ffi::c_void)>,
}

pub type ISzAllocPtr = *const ISzAlloc;

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct See {
    pub summ: u16,
    pub shift: u8,
    pub count: u8,
}

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct State {
    pub symbol: u8,
    pub freq: u8,
    pub successor_0: u16,
    pub successor_1: u16,
}

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct State2 {
    pub symbol: u8,
    pub freq: u8,
}

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct State4 {
    pub successor_0: u16,
    pub successor_1: u16,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union Union2 {
    pub summ_freq: u16,
    pub state2: State2,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union Union4 {
    pub stats: u32,
    pub state4: State4,
}

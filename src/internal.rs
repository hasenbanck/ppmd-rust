pub(crate) mod ppmd7;

pub(crate) mod ppmd8;

const PPMD_INT_BITS: u32 = 7;
const PPMD_PERIOD_BITS: u32 = 7;
const PPMD_BIN_SCALE: u32 = 1 << (PPMD_INT_BITS + PPMD_PERIOD_BITS);

const fn ppmd_get_mean_spec(summ: u32, shift: u32, round: u32) -> u32 {
    (summ + (1 << (shift - round))) >> shift
}

const fn ppmd_get_mean(summ: u32) -> u32 {
    ppmd_get_mean_spec(summ, PPMD_PERIOD_BITS, 2)
}

const fn ppmd_update_prob_1(prob: u32) -> u32 {
    prob - ppmd_get_mean(prob)
}

const PPMD_N1: u32 = 4;
const PPMD_N2: u32 = 4;
const PPMD_N3: u32 = 4;
const PPMD_N4: u32 = (128 + 3 - PPMD_N1 - 2 * PPMD_N2 - 3 * PPMD_N3) / 4;
const PPMD_NUM_INDEXES: u32 = PPMD_N1 + PPMD_N2 + PPMD_N3 + PPMD_N4;

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

#[derive(Copy, Clone, Default)]
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

use crate::{Byte, UInt16, UInt32};

pub mod ppmd7;
pub mod ppmd7dec;
pub mod ppmd7enc;
pub mod ppmd8;
pub mod ppmd8dec;
pub mod ppmd8enc;

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

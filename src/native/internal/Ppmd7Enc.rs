#![allow(
    dead_code,
    mutable_transmutes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]

use super::ppmd7::*;
use super::*;

pub unsafe fn Ppmd7z_Init_RangeEnc(mut p: *mut CPpmd7) {
    (*p).rc.enc.Low = 0 as std::ffi::c_int as UInt64;
    (*p).rc.enc.Range = 0xFFFFFFFF as std::ffi::c_uint;
    (*p).rc.enc.Cache = 0 as std::ffi::c_int as Byte;
    (*p).rc.enc.CacheSize = 1 as std::ffi::c_int as UInt64;
}

// TODO that didn't happen before!
#[allow(arithmetic_overflow)]
#[inline(never)]
unsafe fn Ppmd7z_RangeEnc_ShiftLow(mut p: *mut CPpmd7) {
    if ((*p).rc.enc.Low as UInt32) < 0xFF000000 as std::ffi::c_uint || ((*p).rc.enc.Low >> 32) != 0
    {
        let mut temp: Byte = (*p).rc.enc.Cache;
        loop {
            ((*(*p).rc.enc.Stream).Write).expect("non-null function pointer")(
                (*p).rc.enc.Stream,
                (temp as std::ffi::c_int + ((*p).rc.enc.Low >> 32) as Byte as std::ffi::c_int)
                    as Byte,
            );
            temp = 0xFF as std::ffi::c_int as Byte;
            (*p).rc.enc.CacheSize = ((*p).rc.enc.CacheSize).wrapping_sub(1);
            if !((*p).rc.enc.CacheSize != 0 as std::ffi::c_int as UInt64) {
                break;
            }
        }
        (*p).rc.enc.Cache = ((*p).rc.enc.Low as UInt32 >> 24 as std::ffi::c_int) as Byte;
    }
    (*p).rc.enc.CacheSize = ((*p).rc.enc.CacheSize).wrapping_add(1);
    (*p).rc.enc.CacheSize;
    (*p).rc.enc.Low = (((*p).rc.enc.Low as UInt32) << 8 as std::ffi::c_int) as UInt64;
}

#[inline(always)]
unsafe fn Ppmd7z_RangeEnc_Encode(mut p: *mut CPpmd7, mut start: UInt32, mut size: UInt32) {
    (*p).rc.enc.Low = ((*p).rc.enc.Low).wrapping_add((start * (*p).rc.enc.Range) as UInt64);
    (*p).rc.enc.Range = (*p).rc.enc.Range * size;
}

pub unsafe fn Ppmd7z_Flush_RangeEnc(mut p: *mut CPpmd7) {
    let mut i: std::ffi::c_uint = 0;
    i = 0 as std::ffi::c_int as std::ffi::c_uint;
    while i < 5 as std::ffi::c_int as std::ffi::c_uint {
        Ppmd7z_RangeEnc_ShiftLow(p);
        i = i.wrapping_add(1);
        i;
    }
}

#[inline(always)]
pub unsafe fn Ppmd7z_EncodeSymbol(mut p: *mut CPpmd7, mut symbol: std::ffi::c_int) {
    let mut charMask: [size_t; 32] = [0; 32];
    if (*(*p).MinContext).NumStats as std::ffi::c_int != 1 as std::ffi::c_int {
        let mut s: *mut CPpmd_State = ((*p).Base).offset((*(*p).MinContext).Union4.Stats as isize)
            as *mut std::ffi::c_void as *mut CPpmd_State;
        let mut sum: UInt32 = 0;
        let mut i: std::ffi::c_uint = 0;
        (*p).rc.enc.Range = (*p).rc.enc.Range / (*(*p).MinContext).Union2.SummFreq as UInt32;
        if (*s).Symbol as std::ffi::c_int == symbol {
            Ppmd7z_RangeEnc_Encode(p, 0 as std::ffi::c_int as UInt32, (*s).Freq as UInt32);
            if (*p).rc.enc.Range < (1 as std::ffi::c_int as UInt32) << 24 as std::ffi::c_int {
                (*p).rc.enc.Range <<= 8 as std::ffi::c_int;
                Ppmd7z_RangeEnc_ShiftLow(p);
                if (*p).rc.enc.Range < (1 as std::ffi::c_int as UInt32) << 24 as std::ffi::c_int {
                    (*p).rc.enc.Range <<= 8 as std::ffi::c_int;
                    Ppmd7z_RangeEnc_ShiftLow(p);
                }
            }
            (*p).FoundState = s;
            Ppmd7_Update1_0(p);
            return;
        }
        (*p).PrevSuccess = 0 as std::ffi::c_int as std::ffi::c_uint;
        sum = (*s).Freq as UInt32;
        i = ((*(*p).MinContext).NumStats as std::ffi::c_uint)
            .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint);
        loop {
            s = s.offset(1);
            if (*s).Symbol as std::ffi::c_int == symbol {
                Ppmd7z_RangeEnc_Encode(p, sum, (*s).Freq as UInt32);
                if (*p).rc.enc.Range < (1 as std::ffi::c_int as UInt32) << 24 as std::ffi::c_int {
                    (*p).rc.enc.Range <<= 8 as std::ffi::c_int;
                    Ppmd7z_RangeEnc_ShiftLow(p);
                    if (*p).rc.enc.Range < (1 as std::ffi::c_int as UInt32) << 24 as std::ffi::c_int
                    {
                        (*p).rc.enc.Range <<= 8 as std::ffi::c_int;
                        Ppmd7z_RangeEnc_ShiftLow(p);
                    }
                }
                (*p).FoundState = s;
                Ppmd7_Update1(p);
                return;
            }
            sum = sum.wrapping_add((*s).Freq as UInt32);
            i = i.wrapping_sub(1);
            if !(i != 0) {
                break;
            }
        }
        Ppmd7z_RangeEnc_Encode(
            p,
            sum,
            ((*(*p).MinContext).Union2.SummFreq as UInt32).wrapping_sub(sum),
        );
        (*p).HiBitsFlag = ((*(*p).FoundState).Symbol as std::ffi::c_uint)
            .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
            >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
            & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint;
        let mut z: size_t = 0;
        z = 0 as std::ffi::c_int as size_t;
        while z
            < (256 as std::ffi::c_int as usize)
                .wrapping_div(::core::mem::size_of::<size_t>() as usize)
        {
            charMask[z.wrapping_add(0 as std::ffi::c_int as size_t) as usize] =
                !(0 as std::ffi::c_int as size_t);
            charMask[z.wrapping_add(1 as std::ffi::c_int as size_t) as usize] =
                charMask[z.wrapping_add(0 as std::ffi::c_int as size_t) as usize];
            charMask[z.wrapping_add(2 as std::ffi::c_int as size_t) as usize] =
                charMask[z.wrapping_add(1 as std::ffi::c_int as size_t) as usize];
            charMask[z.wrapping_add(3 as std::ffi::c_int as size_t) as usize] =
                charMask[z.wrapping_add(2 as std::ffi::c_int as size_t) as usize];
            charMask[z.wrapping_add(4 as std::ffi::c_int as size_t) as usize] =
                charMask[z.wrapping_add(3 as std::ffi::c_int as size_t) as usize];
            charMask[z.wrapping_add(5 as std::ffi::c_int as size_t) as usize] =
                charMask[z.wrapping_add(4 as std::ffi::c_int as size_t) as usize];
            charMask[z.wrapping_add(6 as std::ffi::c_int as size_t) as usize] =
                charMask[z.wrapping_add(5 as std::ffi::c_int as size_t) as usize];
            charMask[z.wrapping_add(7 as std::ffi::c_int as size_t) as usize] =
                charMask[z.wrapping_add(6 as std::ffi::c_int as size_t) as usize];
            z = z.wrapping_add(8 as std::ffi::c_int as size_t);
        }
        let mut s2: *mut CPpmd_State = ((*p).Base).offset((*(*p).MinContext).Union4.Stats as isize)
            as *mut std::ffi::c_void as *mut CPpmd_State;
        *(charMask.as_mut_ptr() as *mut Byte).offset((*s).Symbol as isize) =
            0 as std::ffi::c_int as Byte;
        loop {
            let sym0: std::ffi::c_uint =
                (*s2.offset(0 as std::ffi::c_int as isize)).Symbol as std::ffi::c_uint;
            let sym1: std::ffi::c_uint =
                (*s2.offset(1 as std::ffi::c_int as isize)).Symbol as std::ffi::c_uint;
            s2 = s2.offset(2 as std::ffi::c_int as isize);
            *(charMask.as_mut_ptr() as *mut Byte).offset(sym0 as isize) =
                0 as std::ffi::c_int as Byte;
            *(charMask.as_mut_ptr() as *mut Byte).offset(sym1 as isize) =
                0 as std::ffi::c_int as Byte;
            if !(s2 < s) {
                break;
            }
        }
    } else {
        (*p).HiBitsFlag = ((*(*p).FoundState).Symbol as std::ffi::c_uint)
            .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
            >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
            & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint;
        let mut prob: *mut UInt16 = &mut *(*((*p).BinSumm).as_mut_ptr().offset(
            ((*(&mut (*(*p).MinContext).Union2 as *mut C2RustUnnamed_0 as *mut CPpmd_State)).Freq
                as std::ffi::c_uint as size_t)
                .wrapping_sub(1 as std::ffi::c_int as size_t) as isize,
        ))
        .as_mut_ptr()
        .offset(
            ((*p).PrevSuccess)
                .wrapping_add(
                    ((*p).RunLength >> 26 as std::ffi::c_int & 0x20 as std::ffi::c_int)
                        as std::ffi::c_uint,
                )
                .wrapping_add(
                    *((*p).NS2BSIndx).as_mut_ptr().offset(
                        ((*(((*p).Base).offset((*(*p).MinContext).Suffix as isize)
                            as *mut std::ffi::c_void
                            as *mut CPpmd7_Context))
                            .NumStats as size_t)
                            .wrapping_sub(1 as std::ffi::c_int as size_t)
                            as isize,
                    ) as std::ffi::c_uint,
                )
                .wrapping_add(
                    ((*(&mut (*(*p).MinContext).Union2 as *mut C2RustUnnamed_0 as *mut CPpmd_State))
                        .Symbol as std::ffi::c_uint)
                        .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
                        >> 8 as std::ffi::c_int - 4 as std::ffi::c_int
                        & ((1 as std::ffi::c_int) << 4 as std::ffi::c_int) as std::ffi::c_uint,
                )
                .wrapping_add((*p).HiBitsFlag) as isize,
        ) as *mut UInt16;
        let mut s_0: *mut CPpmd_State =
            &mut (*(*p).MinContext).Union2 as *mut C2RustUnnamed_0 as *mut CPpmd_State;
        let mut pr: UInt32 = *prob as UInt32;
        let bound: UInt32 = ((*p).rc.enc.Range >> 14 as std::ffi::c_int) * pr;
        pr = pr.wrapping_sub(
            pr.wrapping_add(
                ((1 as std::ffi::c_int) << 7 as std::ffi::c_int - 2 as std::ffi::c_int) as UInt32,
            ) >> 7 as std::ffi::c_int,
        );
        if (*s_0).Symbol as std::ffi::c_int == symbol {
            *prob = pr.wrapping_add(((1 as std::ffi::c_int) << 7 as std::ffi::c_int) as UInt32)
                as UInt16;
            (*p).rc.enc.Range = bound;
            if (*p).rc.enc.Range < (1 as std::ffi::c_int as UInt32) << 24 as std::ffi::c_int {
                (*p).rc.enc.Range <<= 8 as std::ffi::c_int;
                Ppmd7z_RangeEnc_ShiftLow(p);
            }
            let freq: std::ffi::c_uint = (*s_0).Freq as std::ffi::c_uint;
            let mut c: *mut CPpmd7_Context = ((*p).Base).offset(
                ((*s_0).Successor_0 as UInt32
                    | ((*s_0).Successor_1 as UInt32) << 16 as std::ffi::c_int)
                    as isize,
            ) as *mut std::ffi::c_void
                as *mut CPpmd7_Context;
            (*p).FoundState = s_0;
            (*p).PrevSuccess = 1 as std::ffi::c_int as std::ffi::c_uint;
            (*p).RunLength += 1;
            (*p).RunLength;
            (*s_0).Freq = freq.wrapping_add(
                (freq < 128 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int
                    as std::ffi::c_uint,
            ) as Byte;
            if (*p).OrderFall == 0 as std::ffi::c_int as std::ffi::c_uint
                && c as *const Byte > (*p).Text as *const Byte
            {
                (*p).MinContext = c;
                (*p).MaxContext = (*p).MinContext;
            } else {
                Ppmd7_UpdateModel(p);
            }
            return;
        }
        *prob = pr as UInt16;
        (*p).InitEsc = (*p).ExpEscape[(pr >> 10 as std::ffi::c_int) as usize] as std::ffi::c_uint;
        (*p).rc.enc.Low = ((*p).rc.enc.Low).wrapping_add(bound as UInt64);
        (*p).rc.enc.Range = ((*p).rc.enc.Range).wrapping_sub(bound);
        let mut z_0: size_t = 0;
        z_0 = 0 as std::ffi::c_int as size_t;
        while z_0
            < (256 as std::ffi::c_int as usize)
                .wrapping_div(::core::mem::size_of::<size_t>() as usize)
        {
            charMask[z_0.wrapping_add(0 as std::ffi::c_int as size_t) as usize] =
                !(0 as std::ffi::c_int as size_t);
            charMask[z_0.wrapping_add(1 as std::ffi::c_int as size_t) as usize] =
                charMask[z_0.wrapping_add(0 as std::ffi::c_int as size_t) as usize];
            charMask[z_0.wrapping_add(2 as std::ffi::c_int as size_t) as usize] =
                charMask[z_0.wrapping_add(1 as std::ffi::c_int as size_t) as usize];
            charMask[z_0.wrapping_add(3 as std::ffi::c_int as size_t) as usize] =
                charMask[z_0.wrapping_add(2 as std::ffi::c_int as size_t) as usize];
            charMask[z_0.wrapping_add(4 as std::ffi::c_int as size_t) as usize] =
                charMask[z_0.wrapping_add(3 as std::ffi::c_int as size_t) as usize];
            charMask[z_0.wrapping_add(5 as std::ffi::c_int as size_t) as usize] =
                charMask[z_0.wrapping_add(4 as std::ffi::c_int as size_t) as usize];
            charMask[z_0.wrapping_add(6 as std::ffi::c_int as size_t) as usize] =
                charMask[z_0.wrapping_add(5 as std::ffi::c_int as size_t) as usize];
            charMask[z_0.wrapping_add(7 as std::ffi::c_int as size_t) as usize] =
                charMask[z_0.wrapping_add(6 as std::ffi::c_int as size_t) as usize];
            z_0 = z_0.wrapping_add(8 as std::ffi::c_int as size_t);
        }
        *(charMask.as_mut_ptr() as *mut Byte).offset((*s_0).Symbol as isize) =
            0 as std::ffi::c_int as Byte;
        (*p).PrevSuccess = 0 as std::ffi::c_int as std::ffi::c_uint;
    }
    loop {
        let mut see: *mut CPpmd_See = 0 as *mut CPpmd_See;
        let mut s_1: *mut CPpmd_State = 0 as *mut CPpmd_State;
        let mut sum_0: UInt32 = 0;
        let mut escFreq: UInt32 = 0;
        let mut mc: *mut CPpmd7_Context = 0 as *mut CPpmd7_Context;
        let mut i_0: std::ffi::c_uint = 0;
        let mut numMasked: std::ffi::c_uint = 0;
        if (*p).rc.enc.Range < (1 as std::ffi::c_int as UInt32) << 24 as std::ffi::c_int {
            (*p).rc.enc.Range <<= 8 as std::ffi::c_int;
            Ppmd7z_RangeEnc_ShiftLow(p);
            if (*p).rc.enc.Range < (1 as std::ffi::c_int as UInt32) << 24 as std::ffi::c_int {
                (*p).rc.enc.Range <<= 8 as std::ffi::c_int;
                Ppmd7z_RangeEnc_ShiftLow(p);
            }
        }
        mc = (*p).MinContext;
        numMasked = (*mc).NumStats as std::ffi::c_uint;
        loop {
            (*p).OrderFall = ((*p).OrderFall).wrapping_add(1);
            (*p).OrderFall;
            if (*mc).Suffix == 0 {
                return;
            }
            mc = ((*p).Base).offset((*mc).Suffix as isize) as *mut std::ffi::c_void
                as *mut CPpmd7_Context;
            i_0 = (*mc).NumStats as std::ffi::c_uint;
            if !(i_0 == numMasked) {
                break;
            }
        }
        (*p).MinContext = mc;
        if i_0 != 256 as std::ffi::c_int as std::ffi::c_uint {
            let mut nonMasked: std::ffi::c_uint = i_0.wrapping_sub(numMasked);
            see = ((*p).See[(*p).NS2Indx
                [(nonMasked as size_t).wrapping_sub(1 as std::ffi::c_int as size_t) as usize]
                as std::ffi::c_uint as usize])
                .as_mut_ptr()
                .offset((*p).HiBitsFlag as isize)
                .offset(
                    (nonMasked
                        < ((*(((*p).Base).offset((*mc).Suffix as isize) as *mut std::ffi::c_void
                            as *mut CPpmd7_Context))
                            .NumStats as std::ffi::c_uint)
                            .wrapping_sub(i_0)) as std::ffi::c_int as isize,
                )
                .offset((2 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(
                    (((*mc).Union2.SummFreq as std::ffi::c_uint)
                        < (11 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(i_0))
                        as std::ffi::c_int as std::ffi::c_uint,
                ) as isize)
                .offset(
                    (4 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(
                        (numMasked > nonMasked) as std::ffi::c_int as std::ffi::c_uint,
                    ) as isize,
                );
            let mut summ: std::ffi::c_uint = (*see).Summ as std::ffi::c_uint;
            let mut r: std::ffi::c_uint = summ >> (*see).Shift as std::ffi::c_int;
            (*see).Summ = summ.wrapping_sub(r) as UInt16;
            escFreq = r.wrapping_add(
                (r == 0 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int
                    as std::ffi::c_uint,
            );
        } else {
            see = &mut (*p).DummySee;
            escFreq = 1 as std::ffi::c_int as UInt32;
        }
        s_1 = ((*p).Base).offset((*mc).Union4.Stats as isize) as *mut std::ffi::c_void
            as *mut CPpmd_State;
        sum_0 = 0 as std::ffi::c_int as UInt32;
        loop {
            let cur: std::ffi::c_uint = (*s_1).Symbol as std::ffi::c_uint;
            if cur as std::ffi::c_int == symbol {
                let low: UInt32 = sum_0;
                let freq_0: UInt32 = (*s_1).Freq as UInt32;
                let mut num2: std::ffi::c_uint = 0;
                if ((*see).Shift as std::ffi::c_int) < 7 as std::ffi::c_int && {
                    (*see).Count = ((*see).Count).wrapping_sub(1);
                    (*see).Count as std::ffi::c_int == 0 as std::ffi::c_int
                } {
                    (*see).Summ =
                        (((*see).Summ as std::ffi::c_int) << 1 as std::ffi::c_int) as UInt16;
                    let fresh0 = (*see).Shift;
                    (*see).Shift = ((*see).Shift).wrapping_add(1);
                    (*see).Count = ((3 as std::ffi::c_int) << fresh0 as std::ffi::c_int) as Byte;
                }
                (*p).FoundState = s_1;
                sum_0 = sum_0.wrapping_add(escFreq);
                num2 = i_0.wrapping_div(2 as std::ffi::c_int as std::ffi::c_uint);
                i_0 &= 1 as std::ffi::c_int as std::ffi::c_uint;
                sum_0 =
                    sum_0.wrapping_add(freq_0 & (0 as std::ffi::c_int as UInt32).wrapping_sub(i_0));
                if num2 != 0 as std::ffi::c_int as std::ffi::c_uint {
                    s_1 = s_1.offset(i_0 as isize);
                    loop {
                        let sym0_0: std::ffi::c_uint =
                            (*s_1.offset(0 as std::ffi::c_int as isize)).Symbol as std::ffi::c_uint;
                        let sym1_0: std::ffi::c_uint =
                            (*s_1.offset(1 as std::ffi::c_int as isize)).Symbol as std::ffi::c_uint;
                        s_1 = s_1.offset(2 as std::ffi::c_int as isize);
                        sum_0 = (sum_0 as std::ffi::c_uint).wrapping_add(
                            (*s_1.offset(-(2 as std::ffi::c_int) as isize)).Freq
                                as std::ffi::c_uint
                                & *(charMask.as_mut_ptr() as *mut Byte).offset(sym0_0 as isize)
                                    as std::ffi::c_uint,
                        ) as UInt32 as UInt32;
                        sum_0 = (sum_0 as std::ffi::c_uint).wrapping_add(
                            (*s_1.offset(-(1 as std::ffi::c_int) as isize)).Freq
                                as std::ffi::c_uint
                                & *(charMask.as_mut_ptr() as *mut Byte).offset(sym1_0 as isize)
                                    as std::ffi::c_uint,
                        ) as UInt32 as UInt32;
                        num2 = num2.wrapping_sub(1);
                        if !(num2 != 0) {
                            break;
                        }
                    }
                }
                (*p).rc.enc.Range = (*p).rc.enc.Range / sum_0;
                Ppmd7z_RangeEnc_Encode(p, low, freq_0);
                if (*p).rc.enc.Range < (1 as std::ffi::c_int as UInt32) << 24 as std::ffi::c_int {
                    (*p).rc.enc.Range <<= 8 as std::ffi::c_int;
                    Ppmd7z_RangeEnc_ShiftLow(p);
                    if (*p).rc.enc.Range < (1 as std::ffi::c_int as UInt32) << 24 as std::ffi::c_int
                    {
                        (*p).rc.enc.Range <<= 8 as std::ffi::c_int;
                        Ppmd7z_RangeEnc_ShiftLow(p);
                    }
                }
                Ppmd7_Update2(p);
                return;
            }
            sum_0 = (sum_0 as std::ffi::c_uint).wrapping_add(
                (*s_1).Freq as std::ffi::c_uint
                    & *(charMask.as_mut_ptr() as *mut Byte).offset(cur as isize)
                        as std::ffi::c_uint,
            ) as UInt32 as UInt32;
            s_1 = s_1.offset(1);
            s_1;
            i_0 = i_0.wrapping_sub(1);
            if !(i_0 != 0) {
                break;
            }
        }
        let total: UInt32 = sum_0.wrapping_add(escFreq);
        (*see).Summ = ((*see).Summ as UInt32).wrapping_add(total) as UInt16;
        (*p).rc.enc.Range = (*p).rc.enc.Range / total;
        Ppmd7z_RangeEnc_Encode(p, sum_0, escFreq);
        let mut s2_0: *const CPpmd_State = ((*p).Base)
            .offset((*(*p).MinContext).Union4.Stats as isize)
            as *mut std::ffi::c_void as *mut CPpmd_State;
        s_1 = s_1.offset(-1);
        s_1;
        *(charMask.as_mut_ptr() as *mut Byte).offset((*s_1).Symbol as isize) =
            0 as std::ffi::c_int as Byte;
        loop {
            let sym0_1: std::ffi::c_uint =
                (*s2_0.offset(0 as std::ffi::c_int as isize)).Symbol as std::ffi::c_uint;
            let sym1_1: std::ffi::c_uint =
                (*s2_0.offset(1 as std::ffi::c_int as isize)).Symbol as std::ffi::c_uint;
            s2_0 = s2_0.offset(2 as std::ffi::c_int as isize);
            *(charMask.as_mut_ptr() as *mut Byte).offset(sym0_1 as isize) =
                0 as std::ffi::c_int as Byte;
            *(charMask.as_mut_ptr() as *mut Byte).offset(sym1_1 as isize) =
                0 as std::ffi::c_int as Byte;
            if !(s2_0 < s_1 as *const CPpmd_State) {
                break;
            }
        }
    }
}

pub unsafe fn Ppmd7z_EncodeSymbols(mut p: *mut CPpmd7, mut buf: *const Byte, mut lim: *const Byte) {
    while buf < lim {
        Ppmd7z_EncodeSymbol(p, *buf as std::ffi::c_int);
        buf = buf.offset(1);
        buf;
    }
}

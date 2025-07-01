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

pub unsafe fn Ppmd7z_RangeDec_Init(mut p: *mut CPpmd7_RangeDec) -> i32 {
    let mut i: std::ffi::c_uint = 0;
    (*p).Code = 0 as std::ffi::c_int as u32;
    (*p).Range = 0xFFFFFFFF as std::ffi::c_uint;
    if ((*(*p).Stream).Read).expect("non-null function pointer")((*p).Stream) as std::ffi::c_int
        != 0 as std::ffi::c_int
    {
        return 0 as std::ffi::c_int;
    }
    i = 0 as std::ffi::c_int as std::ffi::c_uint;
    while i < 4 as std::ffi::c_int as std::ffi::c_uint {
        (*p).Code = (*p).Code << 8 as std::ffi::c_int
            | ((*(*p).Stream).Read).expect("non-null function pointer")((*p).Stream) as u32;
        i = i.wrapping_add(1);
        i;
    }
    return ((*p).Code < 0xFFFFFFFF as std::ffi::c_uint) as std::ffi::c_int;
}

#[inline(always)]
unsafe fn Ppmd7z_RD_Decode(mut p: *mut CPpmd7, mut start: u32, mut size: u32) {
    (*p).rc.dec.Code = ((*p).rc.dec.Code).wrapping_sub(start * (*p).rc.dec.Range);
    (*p).rc.dec.Range = (*p).rc.dec.Range * size;
}

pub unsafe fn Ppmd7z_DecodeSymbol(mut p: *mut CPpmd7) -> std::ffi::c_int {
    let mut charMask: [usize; 32] = [0; 32];
    if (*(*p).MinContext).NumStats as std::ffi::c_int != 1 as std::ffi::c_int {
        let mut s: *mut CPpmd_State = ((*p).Base).offset((*(*p).MinContext).Union4.Stats as isize)
            as *mut std::ffi::c_void as *mut CPpmd_State;
        let mut i: std::ffi::c_uint = 0;
        let mut count: u32 = 0;
        let mut hiCnt: u32 = 0;
        let summFreq: u32 = (*(*p).MinContext).Union2.SummFreq as u32;
        (*p).rc.dec.Range = (*p).rc.dec.Range / summFreq;
        count = (*p).rc.dec.Code / (*p).rc.dec.Range;
        hiCnt = count;
        count = count.wrapping_sub((*s).Freq as u32);
        if (count as i32) < 0 as std::ffi::c_int {
            let mut sym: u8 = 0;
            Ppmd7z_RD_Decode(p, 0 as std::ffi::c_int as u32, (*s).Freq as u32);
            if (*p).rc.dec.Range < (1 as std::ffi::c_int as u32) << 24 as std::ffi::c_int {
                (*p).rc.dec.Code = (*p).rc.dec.Code << 8 as std::ffi::c_int
                    | ((*(*p).rc.dec.Stream).Read).expect("non-null function pointer")(
                        (*p).rc.dec.Stream,
                    ) as u32;
                (*p).rc.dec.Range <<= 8 as std::ffi::c_int;
                if (*p).rc.dec.Range < (1 as std::ffi::c_int as u32) << 24 as std::ffi::c_int {
                    (*p).rc.dec.Code = (*p).rc.dec.Code << 8 as std::ffi::c_int
                        | ((*(*p).rc.dec.Stream).Read).expect("non-null function pointer")(
                            (*p).rc.dec.Stream,
                        ) as u32;
                    (*p).rc.dec.Range <<= 8 as std::ffi::c_int;
                }
            }
            (*p).FoundState = s;
            sym = (*s).Symbol;
            Ppmd7_Update1_0(p);
            return sym as std::ffi::c_int;
        }
        (*p).PrevSuccess = 0 as std::ffi::c_int as std::ffi::c_uint;
        i = ((*(*p).MinContext).NumStats as std::ffi::c_uint)
            .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint);
        loop {
            s = s.offset(1);
            count = count.wrapping_sub((*s).Freq as u32);
            if (count as i32) < 0 as std::ffi::c_int {
                let mut sym_0: u8 = 0;
                Ppmd7z_RD_Decode(
                    p,
                    hiCnt.wrapping_sub(count).wrapping_sub((*s).Freq as u32),
                    (*s).Freq as u32,
                );
                if (*p).rc.dec.Range < (1 as std::ffi::c_int as u32) << 24 as std::ffi::c_int {
                    (*p).rc.dec.Code = (*p).rc.dec.Code << 8 as std::ffi::c_int
                        | ((*(*p).rc.dec.Stream).Read).expect("non-null function pointer")(
                            (*p).rc.dec.Stream,
                        ) as u32;
                    (*p).rc.dec.Range <<= 8 as std::ffi::c_int;
                    if (*p).rc.dec.Range < (1 as std::ffi::c_int as u32) << 24 as std::ffi::c_int {
                        (*p).rc.dec.Code = (*p).rc.dec.Code << 8 as std::ffi::c_int
                            | ((*(*p).rc.dec.Stream).Read).expect("non-null function pointer")(
                                (*p).rc.dec.Stream,
                            ) as u32;
                        (*p).rc.dec.Range <<= 8 as std::ffi::c_int;
                    }
                }
                (*p).FoundState = s;
                sym_0 = (*s).Symbol;
                Ppmd7_Update1(p);
                return sym_0 as std::ffi::c_int;
            }
            i = i.wrapping_sub(1);
            if !(i != 0) {
                break;
            }
        }
        if hiCnt >= summFreq {
            return -(2 as std::ffi::c_int);
        }
        hiCnt = hiCnt.wrapping_sub(count);
        Ppmd7z_RD_Decode(p, hiCnt, summFreq.wrapping_sub(hiCnt));
        (*p).HiBitsFlag = ((*(*p).FoundState).Symbol as std::ffi::c_uint)
            .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
            >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
            & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint;
        let mut z: usize = 0;
        z = 0 as std::ffi::c_int as usize;
        while z
            < (256 as std::ffi::c_int as usize)
                .wrapping_div(::core::mem::size_of::<usize>() as usize)
        {
            charMask[z.wrapping_add(0 as std::ffi::c_int as usize) as usize] =
                !(0 as std::ffi::c_int as usize);
            charMask[z.wrapping_add(1 as std::ffi::c_int as usize) as usize] =
                charMask[z.wrapping_add(0 as std::ffi::c_int as usize) as usize];
            charMask[z.wrapping_add(2 as std::ffi::c_int as usize) as usize] =
                charMask[z.wrapping_add(1 as std::ffi::c_int as usize) as usize];
            charMask[z.wrapping_add(3 as std::ffi::c_int as usize) as usize] =
                charMask[z.wrapping_add(2 as std::ffi::c_int as usize) as usize];
            charMask[z.wrapping_add(4 as std::ffi::c_int as usize) as usize] =
                charMask[z.wrapping_add(3 as std::ffi::c_int as usize) as usize];
            charMask[z.wrapping_add(5 as std::ffi::c_int as usize) as usize] =
                charMask[z.wrapping_add(4 as std::ffi::c_int as usize) as usize];
            charMask[z.wrapping_add(6 as std::ffi::c_int as usize) as usize] =
                charMask[z.wrapping_add(5 as std::ffi::c_int as usize) as usize];
            charMask[z.wrapping_add(7 as std::ffi::c_int as usize) as usize] =
                charMask[z.wrapping_add(6 as std::ffi::c_int as usize) as usize];
            z = z.wrapping_add(8 as std::ffi::c_int as usize);
        }
        let mut s2: *mut CPpmd_State = ((*p).Base).offset((*(*p).MinContext).Union4.Stats as isize)
            as *mut std::ffi::c_void as *mut CPpmd_State;
        *(charMask.as_mut_ptr() as *mut u8).offset((*s).Symbol as isize) =
            0 as std::ffi::c_int as u8;
        loop {
            let sym0: std::ffi::c_uint =
                (*s2.offset(0 as std::ffi::c_int as isize)).Symbol as std::ffi::c_uint;
            let sym1: std::ffi::c_uint =
                (*s2.offset(1 as std::ffi::c_int as isize)).Symbol as std::ffi::c_uint;
            s2 = s2.offset(2 as std::ffi::c_int as isize);
            *(charMask.as_mut_ptr() as *mut u8).offset(sym0 as isize) = 0 as std::ffi::c_int as u8;
            *(charMask.as_mut_ptr() as *mut u8).offset(sym1 as isize) = 0 as std::ffi::c_int as u8;
            if !(s2 < s) {
                break;
            }
        }
    } else {
        let mut s_0: *mut CPpmd_State =
            &mut (*(*p).MinContext).Union2 as *mut C2RustUnnamed_0 as *mut CPpmd_State;
        (*p).HiBitsFlag = ((*(*p).FoundState).Symbol as std::ffi::c_uint)
            .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
            >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
            & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint;
        let mut prob: *mut u16 = &mut *(*((*p).BinSumm).as_mut_ptr().offset(
            ((*(&mut (*(*p).MinContext).Union2 as *mut C2RustUnnamed_0 as *mut CPpmd_State)).Freq
                as std::ffi::c_uint as usize)
                .wrapping_sub(1 as std::ffi::c_int as usize) as isize,
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
                            .NumStats as usize)
                            .wrapping_sub(1 as std::ffi::c_int as usize)
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
        ) as *mut u16;
        let mut pr: u32 = *prob as u32;
        let mut size0: u32 = ((*p).rc.dec.Range >> 14 as std::ffi::c_int) * pr;
        pr = pr.wrapping_sub(
            pr.wrapping_add(
                ((1 as std::ffi::c_int) << 7 as std::ffi::c_int - 2 as std::ffi::c_int) as u32,
            ) >> 7 as std::ffi::c_int,
        );
        if (*p).rc.dec.Code < size0 {
            let mut sym_1: u8 = 0;
            *prob = pr.wrapping_add(((1 as std::ffi::c_int) << 7 as std::ffi::c_int) as u32) as u16;
            (*p).rc.dec.Range = size0;
            if (*p).rc.dec.Range < (1 as std::ffi::c_int as u32) << 24 as std::ffi::c_int {
                (*p).rc.dec.Code = (*p).rc.dec.Code << 8 as std::ffi::c_int
                    | ((*(*p).rc.dec.Stream).Read).expect("non-null function pointer")(
                        (*p).rc.dec.Stream,
                    ) as u32;
                (*p).rc.dec.Range <<= 8 as std::ffi::c_int;
            }
            let mut freq: std::ffi::c_uint = (*s_0).Freq as std::ffi::c_uint;
            let mut c: *mut CPpmd7_Context = ((*p).Base).offset(
                ((*s_0).Successor_0 as u32 | ((*s_0).Successor_1 as u32) << 16 as std::ffi::c_int)
                    as isize,
            ) as *mut std::ffi::c_void
                as *mut CPpmd7_Context;
            sym_1 = (*s_0).Symbol;
            (*p).FoundState = s_0;
            (*p).PrevSuccess = 1 as std::ffi::c_int as std::ffi::c_uint;
            (*p).RunLength += 1;
            (*p).RunLength;
            (*s_0).Freq = freq.wrapping_add(
                (freq < 128 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int
                    as std::ffi::c_uint,
            ) as u8;
            if (*p).OrderFall == 0 as std::ffi::c_int as std::ffi::c_uint
                && c as *const u8 > (*p).Text as *const u8
            {
                (*p).MinContext = c;
                (*p).MaxContext = (*p).MinContext;
            } else {
                Ppmd7_UpdateModel(p);
            }
            return sym_1 as std::ffi::c_int;
        }
        *prob = pr as u16;
        (*p).InitEsc = (*p).ExpEscape[(pr >> 10 as std::ffi::c_int) as usize] as std::ffi::c_uint;
        (*p).rc.dec.Code = ((*p).rc.dec.Code).wrapping_sub(size0);
        (*p).rc.dec.Range = ((*p).rc.dec.Range).wrapping_sub(size0);
        let mut z_0: usize = 0;
        z_0 = 0 as std::ffi::c_int as usize;
        while z_0
            < (256 as std::ffi::c_int as usize)
                .wrapping_div(::core::mem::size_of::<usize>() as usize)
        {
            charMask[z_0.wrapping_add(0 as std::ffi::c_int as usize) as usize] =
                !(0 as std::ffi::c_int as usize);
            charMask[z_0.wrapping_add(1 as std::ffi::c_int as usize) as usize] =
                charMask[z_0.wrapping_add(0 as std::ffi::c_int as usize) as usize];
            charMask[z_0.wrapping_add(2 as std::ffi::c_int as usize) as usize] =
                charMask[z_0.wrapping_add(1 as std::ffi::c_int as usize) as usize];
            charMask[z_0.wrapping_add(3 as std::ffi::c_int as usize) as usize] =
                charMask[z_0.wrapping_add(2 as std::ffi::c_int as usize) as usize];
            charMask[z_0.wrapping_add(4 as std::ffi::c_int as usize) as usize] =
                charMask[z_0.wrapping_add(3 as std::ffi::c_int as usize) as usize];
            charMask[z_0.wrapping_add(5 as std::ffi::c_int as usize) as usize] =
                charMask[z_0.wrapping_add(4 as std::ffi::c_int as usize) as usize];
            charMask[z_0.wrapping_add(6 as std::ffi::c_int as usize) as usize] =
                charMask[z_0.wrapping_add(5 as std::ffi::c_int as usize) as usize];
            charMask[z_0.wrapping_add(7 as std::ffi::c_int as usize) as usize] =
                charMask[z_0.wrapping_add(6 as std::ffi::c_int as usize) as usize];
            z_0 = z_0.wrapping_add(8 as std::ffi::c_int as usize);
        }
        *(charMask.as_mut_ptr() as *mut u8).offset(
            (*(&mut (*(*p).MinContext).Union2 as *mut C2RustUnnamed_0 as *mut CPpmd_State)).Symbol
                as isize,
        ) = 0 as std::ffi::c_int as u8;
        (*p).PrevSuccess = 0 as std::ffi::c_int as std::ffi::c_uint;
    }
    loop {
        let mut s_1: *mut CPpmd_State = 0 as *mut CPpmd_State;
        let mut s2_0: *mut CPpmd_State = 0 as *mut CPpmd_State;
        let mut freqSum: u32 = 0;
        let mut count_0: u32 = 0;
        let mut hiCnt_0: u32 = 0;
        let mut see: *mut CPpmd_See = 0 as *mut CPpmd_See;
        let mut mc: *mut CPpmd7_Context = 0 as *mut CPpmd7_Context;
        let mut numMasked: std::ffi::c_uint = 0;
        if (*p).rc.dec.Range < (1 as std::ffi::c_int as u32) << 24 as std::ffi::c_int {
            (*p).rc.dec.Code = (*p).rc.dec.Code << 8 as std::ffi::c_int
                | ((*(*p).rc.dec.Stream).Read).expect("non-null function pointer")(
                    (*p).rc.dec.Stream,
                ) as u32;
            (*p).rc.dec.Range <<= 8 as std::ffi::c_int;
            if (*p).rc.dec.Range < (1 as std::ffi::c_int as u32) << 24 as std::ffi::c_int {
                (*p).rc.dec.Code = (*p).rc.dec.Code << 8 as std::ffi::c_int
                    | ((*(*p).rc.dec.Stream).Read).expect("non-null function pointer")(
                        (*p).rc.dec.Stream,
                    ) as u32;
                (*p).rc.dec.Range <<= 8 as std::ffi::c_int;
            }
        }
        mc = (*p).MinContext;
        numMasked = (*mc).NumStats as std::ffi::c_uint;
        loop {
            (*p).OrderFall = ((*p).OrderFall).wrapping_add(1);
            (*p).OrderFall;
            if (*mc).Suffix == 0 {
                return -(1 as std::ffi::c_int);
            }
            mc = ((*p).Base).offset((*mc).Suffix as isize) as *mut std::ffi::c_void
                as *mut CPpmd7_Context;
            if !((*mc).NumStats as std::ffi::c_uint == numMasked) {
                break;
            }
        }
        s_1 = ((*p).Base).offset((*mc).Union4.Stats as isize) as *mut std::ffi::c_void
            as *mut CPpmd_State;
        let mut num: std::ffi::c_uint = (*mc).NumStats as std::ffi::c_uint;
        let mut num2: std::ffi::c_uint = num.wrapping_div(2 as std::ffi::c_int as std::ffi::c_uint);
        num &= 1 as std::ffi::c_int as std::ffi::c_uint;
        hiCnt_0 = (*s_1).Freq as u32
            & *(charMask.as_mut_ptr() as *mut u8).offset((*s_1).Symbol as isize) as u32
            & (0 as std::ffi::c_int as u32).wrapping_sub(num);
        s_1 = s_1.offset(num as isize);
        (*p).MinContext = mc;
        loop {
            let sym0_0: std::ffi::c_uint =
                (*s_1.offset(0 as std::ffi::c_int as isize)).Symbol as std::ffi::c_uint;
            let sym1_0: std::ffi::c_uint =
                (*s_1.offset(1 as std::ffi::c_int as isize)).Symbol as std::ffi::c_uint;
            s_1 = s_1.offset(2 as std::ffi::c_int as isize);
            hiCnt_0 = hiCnt_0.wrapping_add(
                (*s_1.offset(-(2 as std::ffi::c_int) as isize)).Freq as u32
                    & *(charMask.as_mut_ptr() as *mut u8).offset(sym0_0 as isize) as u32,
            );
            hiCnt_0 = hiCnt_0.wrapping_add(
                (*s_1.offset(-(1 as std::ffi::c_int) as isize)).Freq as u32
                    & *(charMask.as_mut_ptr() as *mut u8).offset(sym1_0 as isize) as u32,
            );
            num2 = num2.wrapping_sub(1);
            if !(num2 != 0) {
                break;
            }
        }
        see = Ppmd7_MakeEscFreq(p, numMasked, &mut freqSum);
        freqSum = freqSum.wrapping_add(hiCnt_0);
        (*p).rc.dec.Range = (*p).rc.dec.Range / freqSum;
        count_0 = (*p).rc.dec.Code / (*p).rc.dec.Range;
        if count_0 < hiCnt_0 {
            let mut sym_2: u8 = 0;
            s_1 = ((*p).Base).offset((*(*p).MinContext).Union4.Stats as isize)
                as *mut std::ffi::c_void as *mut CPpmd_State;
            hiCnt_0 = count_0;
            loop {
                count_0 = count_0.wrapping_sub(
                    (*s_1).Freq as u32
                        & *(charMask.as_mut_ptr() as *mut u8).offset((*s_1).Symbol as isize) as u32,
                );
                s_1 = s_1.offset(1);
                s_1;
                if (count_0 as i32) < 0 as std::ffi::c_int {
                    break;
                }
            }
            s_1 = s_1.offset(-1);
            s_1;
            Ppmd7z_RD_Decode(
                p,
                hiCnt_0
                    .wrapping_sub(count_0)
                    .wrapping_sub((*s_1).Freq as u32),
                (*s_1).Freq as u32,
            );
            if (*p).rc.dec.Range < (1 as std::ffi::c_int as u32) << 24 as std::ffi::c_int {
                (*p).rc.dec.Code = (*p).rc.dec.Code << 8 as std::ffi::c_int
                    | ((*(*p).rc.dec.Stream).Read).expect("non-null function pointer")(
                        (*p).rc.dec.Stream,
                    ) as u32;
                (*p).rc.dec.Range <<= 8 as std::ffi::c_int;
                if (*p).rc.dec.Range < (1 as std::ffi::c_int as u32) << 24 as std::ffi::c_int {
                    (*p).rc.dec.Code = (*p).rc.dec.Code << 8 as std::ffi::c_int
                        | ((*(*p).rc.dec.Stream).Read).expect("non-null function pointer")(
                            (*p).rc.dec.Stream,
                        ) as u32;
                    (*p).rc.dec.Range <<= 8 as std::ffi::c_int;
                }
            }
            if ((*see).Shift as std::ffi::c_int) < 7 as std::ffi::c_int && {
                (*see).Count = ((*see).Count).wrapping_sub(1);
                (*see).Count as std::ffi::c_int == 0 as std::ffi::c_int
            } {
                (*see).Summ = (((*see).Summ as std::ffi::c_int) << 1 as std::ffi::c_int) as u16;
                let fresh0 = (*see).Shift;
                (*see).Shift = ((*see).Shift).wrapping_add(1);
                (*see).Count = ((3 as std::ffi::c_int) << fresh0 as std::ffi::c_int) as u8;
            }
            (*p).FoundState = s_1;
            sym_2 = (*s_1).Symbol;
            Ppmd7_Update2(p);
            return sym_2 as std::ffi::c_int;
        }
        if count_0 >= freqSum {
            return -(2 as std::ffi::c_int);
        }
        Ppmd7z_RD_Decode(p, hiCnt_0, freqSum.wrapping_sub(hiCnt_0));
        (*see).Summ = ((*see).Summ as u32).wrapping_add(freqSum) as u16;
        s_1 = ((*p).Base).offset((*(*p).MinContext).Union4.Stats as isize) as *mut std::ffi::c_void
            as *mut CPpmd_State;
        s2_0 = s_1.offset((*(*p).MinContext).NumStats as std::ffi::c_int as isize);
        loop {
            *(charMask.as_mut_ptr() as *mut u8).offset((*s_1).Symbol as isize) =
                0 as std::ffi::c_int as u8;
            s_1 = s_1.offset(1);
            s_1;
            if !(s_1 != s2_0) {
                break;
            }
        }
    }
}

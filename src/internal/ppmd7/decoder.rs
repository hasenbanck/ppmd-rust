use super::*;
use crate::{SYM_END, SYM_ERROR};

pub(crate) unsafe fn decode_symbol<R: Read>(
    p: *mut PPMd7<RangeDecoder<R>>,
) -> std::io::Result<std::ffi::c_int> {
    let mut charMask: [usize; 32] = [0; 32];
    if (*(*p).min_context).num_stats as std::ffi::c_int != 1 as std::ffi::c_int {
        let mut s: *mut State = ((*p).base).offset((*(*p).min_context).union4.stats as isize)
            as *mut std::ffi::c_void as *mut State;
        let mut i: std::ffi::c_uint = 0;
        let mut hiCnt: u32 = 0;
        let summFreq: u32 = (*(*p).min_context).union2.summ_freq as u32;
        let mut count = (*p).rc.get_threshold(summFreq);
        hiCnt = count;
        count = count.wrapping_sub((*s).freq as u32);
        if (count as i32) < 0 as std::ffi::c_int {
            (*p).rc.decode_final(0, (*s).freq as u32)?;
            (*p).found_state = s;
            let sym = (*s).symbol;
            update1_0(p);
            return Ok(sym as std::ffi::c_int);
        }
        (*p).prev_success = 0 as std::ffi::c_int as std::ffi::c_uint;
        i = ((*(*p).min_context).num_stats as std::ffi::c_uint)
            .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint);
        loop {
            s = s.offset(1);
            count = count.wrapping_sub((*s).freq as u32);
            if (count as i32) < 0 as std::ffi::c_int {
                let mut sym_0: u8 = 0;
                let freq = (*s).freq as u32;
                (*p).rc
                    .decode_final(hiCnt.wrapping_sub(count).wrapping_sub(freq), freq)?;
                (*p).found_state = s;
                sym_0 = (*s).symbol;
                update1(p);
                return Ok(sym_0 as std::ffi::c_int);
            }
            i = i.wrapping_sub(1);
            if !(i != 0) {
                break;
            }
        }

        if hiCnt >= summFreq {
            return Ok(SYM_ERROR);
        }

        hiCnt = hiCnt.wrapping_sub(count);
        (*p).rc.decode(hiCnt, summFreq.wrapping_sub(hiCnt));

        (*p).hi_bits_flag = ((*(*p).found_state).symbol as std::ffi::c_uint)
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
        let mut s2: *mut State = ((*p).base).offset((*(*p).min_context).union4.stats as isize)
            as *mut std::ffi::c_void as *mut State;
        *(charMask.as_mut_ptr() as *mut u8).offset((*s).symbol as isize) =
            0 as std::ffi::c_int as u8;
        loop {
            let sym0: std::ffi::c_uint =
                (*s2.offset(0 as std::ffi::c_int as isize)).symbol as std::ffi::c_uint;
            let sym1: std::ffi::c_uint =
                (*s2.offset(1 as std::ffi::c_int as isize)).symbol as std::ffi::c_uint;
            s2 = s2.offset(2 as std::ffi::c_int as isize);
            *(charMask.as_mut_ptr() as *mut u8).offset(sym0 as isize) = 0 as std::ffi::c_int as u8;
            *(charMask.as_mut_ptr() as *mut u8).offset(sym1 as isize) = 0 as std::ffi::c_int as u8;
            if !(s2 < s) {
                break;
            }
        }
    } else {
        let s_0: *mut State = &mut (*(*p).min_context).union2 as *mut Union2 as *mut State;
        (*p).hi_bits_flag = ((*(*p).found_state).symbol as std::ffi::c_uint)
            .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
            >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
            & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint;
        let prob: *mut u16 = &mut *(*((*p).bin_summ).as_mut_ptr().offset(
            ((*(&mut (*(*p).min_context).union2 as *mut Union2 as *mut State)).freq
                as std::ffi::c_uint as usize)
                .wrapping_sub(1 as std::ffi::c_int as usize) as isize,
        ))
        .as_mut_ptr()
        .offset(
            ((*p).prev_success)
                .wrapping_add(
                    ((*p).run_length >> 26 as std::ffi::c_int & 0x20 as std::ffi::c_int)
                        as std::ffi::c_uint,
                )
                .wrapping_add(
                    *((*p).ns2bs_index).as_mut_ptr().offset(
                        ((*(((*p).base).offset((*(*p).min_context).suffix as isize)
                            as *mut std::ffi::c_void as *mut Context))
                            .num_stats as usize)
                            .wrapping_sub(1 as std::ffi::c_int as usize)
                            as isize,
                    ) as std::ffi::c_uint,
                )
                .wrapping_add(
                    ((*(&mut (*(*p).min_context).union2 as *mut Union2 as *mut State)).symbol
                        as std::ffi::c_uint)
                        .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
                        >> 8 as std::ffi::c_int - 4 as std::ffi::c_int
                        & ((1 as std::ffi::c_int) << 4 as std::ffi::c_int) as std::ffi::c_uint,
                )
                .wrapping_add((*p).hi_bits_flag) as isize,
        ) as *mut u16;
        let mut pr: u32 = *prob as u32;
        let size0: u32 = ((*p).rc.range >> 14 as std::ffi::c_int) * pr;
        pr = pr.wrapping_sub(
            pr.wrapping_add(
                ((1 as std::ffi::c_int) << 7 as std::ffi::c_int - 2 as std::ffi::c_int) as u32,
            ) >> 7 as std::ffi::c_int,
        );
        if (*p).rc.code < size0 {
            let mut sym_1: u8 = 0;
            *prob = pr.wrapping_add(((1 as std::ffi::c_int) << 7 as std::ffi::c_int) as u32) as u16;

            (*p).rc.decode_bit_0(size0)?;

            let freq: std::ffi::c_uint = (*s_0).freq as std::ffi::c_uint;
            let c: *mut Context = ((*p).base).offset(
                ((*s_0).successor_0 as u32 | ((*s_0).successor_1 as u32) << 16 as std::ffi::c_int)
                    as isize,
            ) as *mut std::ffi::c_void as *mut Context;
            sym_1 = (*s_0).symbol;
            (*p).found_state = s_0;
            (*p).prev_success = 1 as std::ffi::c_int as std::ffi::c_uint;
            (*p).run_length += 1;
            (*p).run_length;
            (*s_0).freq = freq.wrapping_add(
                (freq < 128 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int
                    as std::ffi::c_uint,
            ) as u8;
            if (*p).order_fall == 0 as std::ffi::c_int as std::ffi::c_uint
                && c as *const u8 > (*p).text as *const u8
            {
                (*p).min_context = c;
                (*p).max_context = (*p).min_context;
            } else {
                update_model(p);
            }
            return Ok(sym_1 as std::ffi::c_int);
        }
        *prob = pr as u16;
        (*p).init_esc = (*p).exp_escape[(pr >> 10 as std::ffi::c_int) as usize] as std::ffi::c_uint;

        (*p).rc.decode_bit_1(size0);

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
            (*(&mut (*(*p).min_context).union2 as *mut Union2 as *mut State)).symbol as isize,
        ) = 0 as std::ffi::c_int as u8;
        (*p).prev_success = 0 as std::ffi::c_int as std::ffi::c_uint;
    }
    loop {
        let mut s_1: *mut State = 0 as *mut State;
        let mut s2_0: *mut State = 0 as *mut State;
        let mut freqSum: u32 = 0;
        let mut count_0: u32 = 0;
        let mut hiCnt_0: u32 = 0;
        let mut see: *mut See = 0 as *mut See;
        let mut mc: *mut Context = 0 as *mut Context;
        let mut numMasked: std::ffi::c_uint = 0;
        (*p).rc.normalize_remote()?;
        mc = (*p).min_context;
        numMasked = (*mc).num_stats as std::ffi::c_uint;
        loop {
            (*p).order_fall = ((*p).order_fall).wrapping_add(1);
            (*p).order_fall;
            if (*mc).suffix == 0 {
                return Ok(SYM_END);
            }
            mc = ((*p).base).offset((*mc).suffix as isize) as *mut std::ffi::c_void as *mut Context;
            if !((*mc).num_stats as std::ffi::c_uint == numMasked) {
                break;
            }
        }
        s_1 =
            ((*p).base).offset((*mc).union4.stats as isize) as *mut std::ffi::c_void as *mut State;
        let mut num: std::ffi::c_uint = (*mc).num_stats as std::ffi::c_uint;
        let mut num2: std::ffi::c_uint = num.wrapping_div(2 as std::ffi::c_int as std::ffi::c_uint);
        num &= 1 as std::ffi::c_int as std::ffi::c_uint;
        hiCnt_0 = (*s_1).freq as u32
            & *(charMask.as_mut_ptr() as *mut u8).offset((*s_1).symbol as isize) as u32
            & (0 as std::ffi::c_int as u32).wrapping_sub(num);
        s_1 = s_1.offset(num as isize);
        (*p).min_context = mc;
        loop {
            let sym0_0: std::ffi::c_uint =
                (*s_1.offset(0 as std::ffi::c_int as isize)).symbol as std::ffi::c_uint;
            let sym1_0: std::ffi::c_uint =
                (*s_1.offset(1 as std::ffi::c_int as isize)).symbol as std::ffi::c_uint;
            s_1 = s_1.offset(2 as std::ffi::c_int as isize);
            hiCnt_0 = hiCnt_0.wrapping_add(
                (*s_1.offset(-(2 as std::ffi::c_int) as isize)).freq as u32
                    & *(charMask.as_mut_ptr() as *mut u8).offset(sym0_0 as isize) as u32,
            );
            hiCnt_0 = hiCnt_0.wrapping_add(
                (*s_1.offset(-(1 as std::ffi::c_int) as isize)).freq as u32
                    & *(charMask.as_mut_ptr() as *mut u8).offset(sym1_0 as isize) as u32,
            );
            num2 = num2.wrapping_sub(1);
            if !(num2 != 0) {
                break;
            }
        }
        see = make_esc_freq(p, numMasked, &mut freqSum);
        freqSum = freqSum.wrapping_add(hiCnt_0);

        count_0 = (*p).rc.get_threshold(freqSum);

        if count_0 < hiCnt_0 {
            let mut sym_2: u8 = 0;
            s_1 = ((*p).base).offset((*(*p).min_context).union4.stats as isize)
                as *mut std::ffi::c_void as *mut State;
            hiCnt_0 = count_0;
            loop {
                count_0 = count_0.wrapping_sub(
                    (*s_1).freq as u32
                        & *(charMask.as_mut_ptr() as *mut u8).offset((*s_1).symbol as isize) as u32,
                );
                s_1 = s_1.offset(1);
                s_1;
                if (count_0 as i32) < 0 as std::ffi::c_int {
                    break;
                }
            }
            s_1 = s_1.offset(-1);

            (*p).rc.decode_final(
                hiCnt_0
                    .wrapping_sub(count_0)
                    .wrapping_sub((*s_1).freq as u32),
                (*s_1).freq as u32,
            )?;

            if ((*see).shift as std::ffi::c_int) < 7 as std::ffi::c_int && {
                (*see).count = ((*see).count).wrapping_sub(1);
                (*see).count as std::ffi::c_int == 0 as std::ffi::c_int
            } {
                (*see).summ = (((*see).summ as std::ffi::c_int) << 1 as std::ffi::c_int) as u16;
                let fresh0 = (*see).shift;
                (*see).shift = ((*see).shift).wrapping_add(1);
                (*see).count = ((3 as std::ffi::c_int) << fresh0 as std::ffi::c_int) as u8;
            }
            (*p).found_state = s_1;
            sym_2 = (*s_1).symbol;
            update2(p);
            return Ok(sym_2 as std::ffi::c_int);
        }

        if count_0 >= freqSum {
            return Ok(SYM_ERROR);
        }

        (*p).rc.decode(hiCnt_0, freqSum.wrapping_sub(hiCnt_0));

        (*see).summ = ((*see).summ as u32).wrapping_add(freqSum) as u16;
        s_1 = ((*p).base).offset((*(*p).min_context).union4.stats as isize) as *mut std::ffi::c_void
            as *mut State;
        s2_0 = s_1.offset((*(*p).min_context).num_stats as std::ffi::c_int as isize);
        loop {
            *(charMask.as_mut_ptr() as *mut u8).offset((*s_1).symbol as isize) =
                0 as std::ffi::c_int as u8;
            s_1 = s_1.offset(1);
            s_1;
            if !(s_1 != s2_0) {
                break;
            }
        }
    }
}

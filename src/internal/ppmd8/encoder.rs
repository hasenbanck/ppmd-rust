use super::*;

pub unsafe fn encode_symbol<W: Write>(
    p: *mut PPMd8<RangeEncoder<W>>,
    symbol: std::ffi::c_int,
) -> std::io::Result<()> {
    let mut charMask: [usize; 32] = [0; 32];
    if (*(*p).min_context).num_stats as std::ffi::c_int != 0 as std::ffi::c_int {
        let mut s: *mut State = ((*p).base).offset((*(*p).min_context).union4.stats as isize)
            as *mut std::ffi::c_void as *mut State;
        let mut sum: u32 = 0;
        let mut i: std::ffi::c_uint = 0;
        let mut summFreq: u32 = (*(*p).min_context).union2.summ_freq as u32;

        summFreq = (*p).rc.correct_sum_range(summFreq);

        if (*s).symbol as std::ffi::c_int == symbol {
            (*p).rc.encode_final(0, (*s).freq as u32, summFreq)?;
            (*p).found_state = s;
            update1_0(p);
            return Ok(());
        }
        (*p).prev_success = 0 as std::ffi::c_int as std::ffi::c_uint;
        sum = (*s).freq as u32;
        i = (*(*p).min_context).num_stats as std::ffi::c_uint;
        loop {
            s = s.offset(1);
            if (*s).symbol as std::ffi::c_int == symbol {
                (*p).rc.encode_final(sum, (*s).freq as u32, summFreq)?;
                (*p).found_state = s;
                update1(p);
                return Ok(());
            }
            sum = sum.wrapping_add((*s).freq as u32);
            i = i.wrapping_sub(1);
            if !(i != 0) {
                break;
            }
        }

        (*p).rc.encode(sum, summFreq.wrapping_sub(sum), summFreq);

        let mut z: usize = 0;
        z = 0 as std::ffi::c_int as usize;
        while z < 256usize.wrapping_div(::core::mem::size_of::<usize>()) {
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
        let prob: *mut u16 = &mut *(*((*p).bin_summ).as_mut_ptr().offset(
            *((*p).ns2index).as_mut_ptr().offset(
                ((*(&mut (*(*p).min_context).union2 as *mut Union2 as *mut State)).freq as usize)
                    .wrapping_sub(1 as std::ffi::c_int as usize) as isize,
            ) as isize,
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
                        (*(((*p).base).offset((*(*p).min_context).suffix as isize)
                            as *mut std::ffi::c_void as *mut Context))
                            .num_stats as isize,
                    ) as std::ffi::c_uint,
                )
                .wrapping_add((*(*p).min_context).flags as std::ffi::c_int as std::ffi::c_uint)
                as isize,
        ) as *mut u16;
        let s_0: *mut State = &mut (*(*p).min_context).union2 as *mut Union2 as *mut State;
        let mut pr: u32 = *prob as u32;
        let bound: u32 = ((*p).rc.range >> 14 as std::ffi::c_int) * pr;
        pr = pr.wrapping_sub(
            pr.wrapping_add(
                ((1 as std::ffi::c_int) << 7 as std::ffi::c_int - 2 as std::ffi::c_int) as u32,
            ) >> 7 as std::ffi::c_int,
        );

        if (*s_0).symbol as std::ffi::c_int == symbol {
            *prob = pr.wrapping_add(((1 as std::ffi::c_int) << 7 as std::ffi::c_int) as u32) as u16;
            (*p).rc.range = bound;
            (*p).rc.normalize_remote()?;

            let freq: std::ffi::c_uint = (*s_0).freq as std::ffi::c_uint;
            let c: *mut Context = ((*p).base).offset(
                ((*s_0).successor_0 as u32 | ((*s_0).successor_1 as u32) << 16 as std::ffi::c_int)
                    as isize,
            ) as *mut std::ffi::c_void as *mut Context;
            (*p).found_state = s_0;
            (*p).prev_success = 1 as std::ffi::c_int as std::ffi::c_uint;
            (*p).run_length += 1;
            (*p).run_length;
            (*s_0).freq = freq.wrapping_add(
                (freq < 196 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int
                    as std::ffi::c_uint,
            ) as u8;
            if (*p).order_fall == 0 as std::ffi::c_int as std::ffi::c_uint
                && c as *const u8 >= (*p).units_start as *const u8
            {
                (*p).min_context = c;
                (*p).max_context = (*p).min_context;
            } else {
                update_model(p);
            }
            return Ok(());
        }

        *prob = pr as u16;
        (*p).init_esc = (*p).exp_escape[(pr >> 10 as std::ffi::c_int) as usize] as std::ffi::c_uint;

        (*p).rc.encode_bit_1(bound);

        let mut z_0: usize = 0;
        z_0 = 0 as std::ffi::c_int as usize;
        while z_0 < 256usize.wrapping_div(::core::mem::size_of::<usize>()) {
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
        *(charMask.as_mut_ptr() as *mut u8).offset((*s_0).symbol as isize) =
            0 as std::ffi::c_int as u8;
        (*p).prev_success = 0 as std::ffi::c_int as std::ffi::c_uint;
    }
    loop {
        let mut see: *mut See = 0 as *mut See;
        let mut s_1: *mut State = 0 as *mut State;
        let mut sum_0: u32 = 0;
        let mut escFreq: u32 = 0;
        let mut mc: *mut Context = 0 as *mut Context;
        let mut i_0: std::ffi::c_uint = 0;
        let mut numMasked: std::ffi::c_uint = 0;

        (*p).rc.normalize_remote()?;

        mc = (*p).min_context;
        numMasked = (*mc).num_stats as std::ffi::c_uint;

        loop {
            (*p).order_fall = ((*p).order_fall).wrapping_add(1);
            (*p).order_fall;
            if (*mc).suffix == 0 {
                return Ok(());
            }
            mc = ((*p).base).offset((*mc).suffix as isize) as *mut std::ffi::c_void as *mut Context;
            if !((*mc).num_stats as std::ffi::c_uint == numMasked) {
                break;
            }
        }
        (*p).min_context = mc;
        see = make_esc_freq(p, numMasked, &mut escFreq);
        s_1 = ((*p).base).offset((*(*p).min_context).union4.stats as isize) as *mut std::ffi::c_void
            as *mut State;
        sum_0 = 0 as std::ffi::c_int as u32;
        i_0 = ((*(*p).min_context).num_stats as std::ffi::c_uint)
            .wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint);
        loop {
            let cur: std::ffi::c_uint = (*s_1).symbol as std::ffi::c_uint;
            if cur as std::ffi::c_int == symbol {
                let low: u32 = sum_0;
                let freq_0: u32 = (*s_1).freq as u32;
                let mut num2: std::ffi::c_uint = 0;
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
                sum_0 = sum_0.wrapping_add(escFreq);
                num2 = i_0.wrapping_div(2 as std::ffi::c_int as std::ffi::c_uint);
                i_0 &= 1 as std::ffi::c_int as std::ffi::c_uint;
                sum_0 =
                    sum_0.wrapping_add(freq_0 & (0 as std::ffi::c_int as u32).wrapping_sub(i_0));
                if num2 != 0 as std::ffi::c_int as std::ffi::c_uint {
                    s_1 = s_1.offset(i_0 as isize);
                    loop {
                        let sym0_0: std::ffi::c_uint =
                            (*s_1.offset(0 as std::ffi::c_int as isize)).symbol as std::ffi::c_uint;
                        let sym1_0: std::ffi::c_uint =
                            (*s_1.offset(1 as std::ffi::c_int as isize)).symbol as std::ffi::c_uint;
                        s_1 = s_1.offset(2 as std::ffi::c_int as isize);
                        sum_0 = (sum_0 as std::ffi::c_uint).wrapping_add(
                            (*s_1.offset(-(2 as std::ffi::c_int) as isize)).freq
                                as std::ffi::c_uint
                                & *(charMask.as_mut_ptr() as *mut u8).offset(sym0_0 as isize)
                                    as std::ffi::c_uint,
                        );
                        sum_0 = (sum_0 as std::ffi::c_uint).wrapping_add(
                            (*s_1.offset(-(1 as std::ffi::c_int) as isize)).freq
                                as std::ffi::c_uint
                                & *(charMask.as_mut_ptr() as *mut u8).offset(sym1_0 as isize)
                                    as std::ffi::c_uint,
                        );
                        num2 = num2.wrapping_sub(1);
                        if !(num2 != 0) {
                            break;
                        }
                    }
                }

                sum_0 = (*p).rc.correct_sum_range(sum_0);

                (*p).rc.encode_final(low, freq_0, sum_0)?;
                update2(p);
                return Ok(());
            }
            sum_0 = (sum_0 as std::ffi::c_uint).wrapping_add(
                (*s_1).freq as std::ffi::c_uint
                    & *(charMask.as_mut_ptr() as *mut u8).offset(cur as isize) as std::ffi::c_uint,
            );
            s_1 = s_1.offset(1);
            s_1;
            i_0 = i_0.wrapping_sub(1);
            if !(i_0 != 0) {
                break;
            }
        }

        let mut total: u32 = sum_0.wrapping_add(escFreq);
        (*see).summ = ((*see).summ as u32).wrapping_add(total) as u16;

        total = (*p).rc.correct_sum_range(total);

        (*p).rc.encode(sum_0, total.wrapping_sub(sum_0), total);

        let mut s2_0: *const State = ((*p).base).offset((*(*p).min_context).union4.stats as isize)
            as *mut std::ffi::c_void as *mut State;
        s_1 = s_1.offset(-1);
        s_1;
        *(charMask.as_mut_ptr() as *mut u8).offset((*s_1).symbol as isize) =
            0 as std::ffi::c_int as u8;
        loop {
            let sym0_1: std::ffi::c_uint =
                (*s2_0.offset(0 as std::ffi::c_int as isize)).symbol as std::ffi::c_uint;
            let sym1_1: std::ffi::c_uint =
                (*s2_0.offset(1 as std::ffi::c_int as isize)).symbol as std::ffi::c_uint;
            s2_0 = s2_0.offset(2 as std::ffi::c_int as isize);
            *(charMask.as_mut_ptr() as *mut u8).offset(sym0_1 as isize) =
                0 as std::ffi::c_int as u8;
            *(charMask.as_mut_ptr() as *mut u8).offset(sym1_1 as isize) =
                0 as std::ffi::c_int as u8;
            if !(s2_0 < s_1 as *const State) {
                break;
            }
        }
    }
}

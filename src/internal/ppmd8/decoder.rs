use super::*;
use crate::{SYM_END, SYM_ERROR};

impl<R: Read> PPMd8<RangeDecoder<R>> {
    pub unsafe fn decode_symbol(&mut self) -> std::io::Result<std::ffi::c_int> {
        let mut charMask: [usize; 32] = [0; 32];
        if self.min_context.as_ref().num_stats as std::ffi::c_int != 0 as std::ffi::c_int {
            let mut s = self
                .memory_ptr
                .offset(self.min_context.as_ref().union4.stats as isize)
                .cast::<State>();
            let mut i: std::ffi::c_uint = 0;
            let mut count: u32 = 0;
            let mut hiCnt: u32 = 0;
            let mut summFreq: u32 = self.min_context.as_ref().union2.summ_freq as u32;

            summFreq = self.rc.correct_sum_range(summFreq);

            count = self.rc.get_threshold(summFreq);
            hiCnt = count;

            count = count.wrapping_sub(s.as_ref().freq as u32);
            if (count as i32) < 0 as std::ffi::c_int {
                let mut sym: u8 = 0;
                self.rc.decode_final(0, s.as_ref().freq as u32)?;
                self.found_state = s;
                sym = s.as_ref().symbol;
                self.update1_0();
                return Ok(sym as std::ffi::c_int);
            }
            self.prev_success = 0 as std::ffi::c_int as std::ffi::c_uint;
            i = self.min_context.as_ref().num_stats as std::ffi::c_uint;
            loop {
                s = s.offset(1);
                count = count.wrapping_sub(s.as_ref().freq as u32);
                if (count as i32) < 0 as std::ffi::c_int {
                    let mut sym_0: u8 = 0;
                    let freq = s.as_ref().freq as u32;
                    self.rc
                        .decode_final(hiCnt.wrapping_sub(count).wrapping_sub(freq), freq)?;
                    self.found_state = s;
                    sym_0 = s.as_ref().symbol;
                    self.update1();
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
            self.rc.decode(hiCnt, summFreq.wrapping_sub(hiCnt));

            let mut z: usize = 0;
            z = 0 as std::ffi::c_int as usize;
            while z < 256usize.wrapping_div(size_of::<usize>()) {
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
            let mut s2 = self
                .memory_ptr
                .offset(self.min_context.as_ref().union4.stats as isize)
                .cast::<State>();
            *(charMask.as_mut_ptr() as *mut u8).offset(s.as_ref().symbol as isize) =
                0 as std::ffi::c_int as u8;
            loop {
                let sym0: std::ffi::c_uint =
                    s2.offset(0 as std::ffi::c_int as isize).as_ref().symbol as std::ffi::c_uint;
                let sym1: std::ffi::c_uint =
                    s2.offset(1 as std::ffi::c_int as isize).as_ref().symbol as std::ffi::c_uint;
                s2 = s2.offset(2 as std::ffi::c_int as isize);
                *(charMask.as_mut_ptr() as *mut u8).offset(sym0 as isize) =
                    0 as std::ffi::c_int as u8;
                *(charMask.as_mut_ptr() as *mut u8).offset(sym1 as isize) =
                    0 as std::ffi::c_int as u8;
                if !(s2 < s) {
                    break;
                }
            }
        } else {
            let mut s_0 = NonNull::new_unchecked(addr_of_mut!(self.min_context.as_mut().union2))
                .cast::<State>();
            let prob: *mut u16 = &mut *(*(self.bin_summ).as_mut_ptr().offset(
                *(self.ns2index).as_mut_ptr().offset(
                    ((*(&mut self.min_context.as_mut().union2 as *mut Union2 as *mut State)).freq
                        as usize)
                        .wrapping_sub(1 as std::ffi::c_int as usize) as isize,
                ) as isize,
            ))
            .as_mut_ptr()
            .offset(
                (self.prev_success)
                    .wrapping_add(
                        (self.run_length >> 26 as std::ffi::c_int & 0x20 as std::ffi::c_int)
                            as std::ffi::c_uint,
                    )
                    .wrapping_add(
                        *(self.ns2bs_index).as_mut_ptr().offset(
                            self.memory_ptr
                                .offset(self.min_context.as_ref().suffix as isize)
                                .cast::<Context>()
                                .as_ref()
                                .num_stats as isize,
                        ) as std::ffi::c_uint,
                    )
                    .wrapping_add(
                        self.min_context.as_ref().flags as std::ffi::c_int as std::ffi::c_uint,
                    ) as isize,
            ) as *mut u16;
            let mut pr: u32 = *prob as u32;
            let size0: u32 = (self.rc.range >> 14 as std::ffi::c_int) * pr;
            pr = pr.wrapping_sub(
                pr.wrapping_add(
                    ((1 as std::ffi::c_int) << 7 as std::ffi::c_int - 2 as std::ffi::c_int) as u32,
                ) >> 7 as std::ffi::c_int,
            );
            if self.rc.code < size0 {
                let mut sym_1: u8 = 0;
                *prob =
                    pr.wrapping_add(((1 as std::ffi::c_int) << 7 as std::ffi::c_int) as u32) as u16;
                self.rc.range = size0;
                self.rc.normalize_remote()?;

                let freq: std::ffi::c_uint = s_0.as_ref().freq as std::ffi::c_uint;
                let c = self
                    .memory_ptr
                    .offset(
                        (s_0.as_ref().successor_0 as u32
                            | (s_0.as_ref().successor_1 as u32) << 16 as std::ffi::c_int)
                            as isize,
                    )
                    .cast::<Context>();
                sym_1 = s_0.as_ref().symbol;
                self.found_state = s_0;
                self.prev_success = 1 as std::ffi::c_int as std::ffi::c_uint;
                self.run_length += 1;
                self.run_length;
                s_0.as_mut().freq = freq.wrapping_add(
                    (freq < 196 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int
                        as std::ffi::c_uint,
                ) as u8;
                if self.order_fall == 0 as std::ffi::c_int as std::ffi::c_uint
                    && c.cast() >= self.units_start
                {
                    self.min_context = c;
                    self.max_context = self.min_context;
                } else {
                    self.update_model();
                }
                return Ok(sym_1 as std::ffi::c_int);
            }
            *prob = pr as u16;
            self.init_esc =
                self.exp_escape[(pr >> 10 as std::ffi::c_int) as usize] as std::ffi::c_uint;

            self.rc.decode_bit_1(size0);

            let mut z_0: usize = 0;
            z_0 = 0 as std::ffi::c_int as usize;
            while z_0 < 256usize.wrapping_div(size_of::<usize>()) {
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
                (*(&mut self.min_context.as_mut().union2 as *mut Union2 as *mut State)).symbol
                    as isize,
            ) = 0 as std::ffi::c_int as u8;
            self.prev_success = 0 as std::ffi::c_int as std::ffi::c_uint;
        }
        loop {
            let mut freqSum: u32 = 0;
            let mut count_0: u32 = 0;
            let mut hiCnt_0: u32 = 0;
            let mut freqSum2: u32 = 0;

            let mut numMasked: std::ffi::c_uint = 0;
            self.rc.normalize_remote()?;
            let mut mc = self.min_context;
            numMasked = mc.as_ref().num_stats as std::ffi::c_uint;

            loop {
                self.order_fall = (self.order_fall).wrapping_add(1);
                self.order_fall;
                if mc.as_ref().suffix == 0 {
                    return Ok(SYM_END);
                }
                mc = self.memory_ptr.offset(mc.as_ref().suffix as isize).cast();

                if !(mc.as_ref().num_stats as std::ffi::c_uint == numMasked) {
                    break;
                }
            }

            let mut s_1 = self
                .memory_ptr
                .offset(mc.as_ref().union4.stats as isize)
                .cast::<State>();
            let mut num: std::ffi::c_uint = (mc.as_ref().num_stats as std::ffi::c_uint)
                .wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint);
            let mut num2: std::ffi::c_uint =
                num.wrapping_div(2 as std::ffi::c_int as std::ffi::c_uint);

            num &= 1 as std::ffi::c_int as std::ffi::c_uint;
            hiCnt_0 = s_1.as_ref().freq as u32
                & *(charMask.as_mut_ptr() as *mut u8).offset(s_1.as_ref().symbol as isize) as u32
                & (0 as std::ffi::c_int as u32).wrapping_sub(num);
            s_1 = s_1.offset(num as isize);
            self.min_context = mc;

            loop {
                let sym0_0: std::ffi::c_uint =
                    s_1.offset(0 as std::ffi::c_int as isize).as_ref().symbol as std::ffi::c_uint;
                let sym1_0: std::ffi::c_uint =
                    s_1.offset(1 as std::ffi::c_int as isize).as_ref().symbol as std::ffi::c_uint;
                s_1 = s_1.offset(2 as std::ffi::c_int as isize);
                hiCnt_0 = hiCnt_0.wrapping_add(
                    s_1.offset(-(2 as std::ffi::c_int) as isize).as_ref().freq as u32
                        & *(charMask.as_mut_ptr() as *mut u8).offset(sym0_0 as isize) as u32,
                );
                hiCnt_0 = hiCnt_0.wrapping_add(
                    s_1.offset(-(1 as std::ffi::c_int) as isize).as_ref().freq as u32
                        & *(charMask.as_mut_ptr() as *mut u8).offset(sym1_0 as isize) as u32,
                );
                num2 = num2.wrapping_sub(1);
                if !(num2 != 0) {
                    break;
                }
            }

            let mut see = self.make_esc_freq(numMasked, &mut freqSum);
            freqSum = freqSum.wrapping_add(hiCnt_0);
            let freqSum2 = self.rc.correct_sum_range(freqSum);

            count_0 = self.rc.get_threshold(freqSum2);

            if count_0 < hiCnt_0 {
                let mut sym_2: u8 = 0;
                s_1 = self
                    .memory_ptr
                    .offset(self.min_context.as_ref().union4.stats as isize)
                    .cast();
                hiCnt_0 = count_0;
                loop {
                    count_0 = count_0.wrapping_sub(
                        s_1.as_ref().freq as u32
                            & *(charMask.as_mut_ptr() as *mut u8)
                                .offset(s_1.as_ref().symbol as isize)
                                as u32,
                    );
                    s_1 = s_1.offset(1);
                    if (count_0 as i32) < 0 as std::ffi::c_int {
                        break;
                    }
                }
                s_1 = s_1.offset(-1);
                self.rc.decode_final(
                    hiCnt_0
                        .wrapping_sub(count_0)
                        .wrapping_sub(s_1.as_ref().freq as u32),
                    s_1.as_ref().freq as u32,
                )?;

                if ((*see).shift as std::ffi::c_int) < 7 as std::ffi::c_int && {
                    (*see).count = (*see).count.wrapping_sub(1);
                    (*see).count as std::ffi::c_int == 0 as std::ffi::c_int
                } {
                    (*see).summ = (((*see).summ as std::ffi::c_int) << 1 as std::ffi::c_int) as u16;
                    let fresh0 = (*see).shift;
                    (*see).shift = ((*see).shift).wrapping_add(1);
                    (*see).count = ((3 as std::ffi::c_int) << fresh0 as std::ffi::c_int) as u8;
                }
                self.found_state = s_1;
                sym_2 = s_1.as_ref().symbol;
                self.update2();
                return Ok(sym_2 as std::ffi::c_int);
            }

            if count_0 >= freqSum2 {
                return Ok(SYM_ERROR);
            }

            self.rc.decode(hiCnt_0, freqSum2.wrapping_sub(hiCnt_0));

            (*see).summ = ((*see).summ as u32).wrapping_add(freqSum) as u16;
            s_1 = self
                .memory_ptr
                .offset(self.min_context.as_ref().union4.stats as isize)
                .cast();
            let s2_0 = s_1
                .offset(self.min_context.as_ref().num_stats as std::ffi::c_int as isize)
                .offset(1 as std::ffi::c_int as isize);
            loop {
                *(charMask.as_mut_ptr() as *mut u8).offset(s_1.as_ref().symbol as isize) =
                    0 as std::ffi::c_int as u8;
                s_1 = s_1.offset(1);
                if !(s_1 != s2_0) {
                    break;
                }
            }
        }
    }
}

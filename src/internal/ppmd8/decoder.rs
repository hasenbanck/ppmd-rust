use super::*;
use crate::internal::{PPMD_INT_BITS, ppmd_update_prob_1};
use crate::{SYM_END, SYM_ERROR};

impl<R: Read> Ppmd8<RangeDecoder<R>> {
    pub(crate) unsafe fn decode_symbol(&mut self) -> Result<i32, std::io::Error> {
        unsafe {
            let mut char_mask: [u8; 256];
            if self.min_context.as_ref().num_stats != 0 {
                let mut s = self.get_multi_state_stats(self.min_context);
                let mut summ_freq = self.min_context.as_ref().data.multi_state.summ_freq as u32;

                if summ_freq > self.rc.range {
                    summ_freq = self.rc.range;
                }
                self.rc.range /= summ_freq;
                let mut count = self.rc.code / self.rc.range;
                let mut hi_cnt = count;

                count = count.wrapping_sub(s.as_ref().freq as u32);
                if (count as i32) < 0 {
                    self.rc.decode(0, s.as_ref().freq as u32);
                    while self.rc.low ^ self.rc.low.wrapping_add(self.rc.range) < K_TOP_VALUE
                        || self.rc.range < K_BOT_VALUE && {
                            self.rc.range = 0u32.wrapping_sub(self.rc.low) & (K_BOT_VALUE - 1);
                            1 != 0
                        }
                    {
                        self.rc.code = self.rc.code << 8 | self.rc.read_byte()?;
                        self.rc.range <<= 8;
                        self.rc.low <<= 8;
                    }
                    self.found_state = s;
                    let sym = s.as_ref().symbol;
                    self.update1_0();
                    return Ok(sym as i32);
                }
                self.prev_success = 0;
                let mut i = self.min_context.as_ref().num_stats as u32;
                loop {
                    s = s.offset(1);
                    count = count.wrapping_sub(s.as_ref().freq as u32);
                    if (count as i32) < 0 {
                        self.rc.decode(
                            hi_cnt
                                .wrapping_sub(count)
                                .wrapping_sub(s.as_ref().freq as u32),
                            s.as_ref().freq as u32,
                        );
                        while self.rc.low ^ self.rc.low.wrapping_add(self.rc.range) < K_TOP_VALUE
                            || self.rc.range < K_BOT_VALUE && {
                                self.rc.range = 0u32.wrapping_sub(self.rc.low) & (K_BOT_VALUE - 1);
                                1 != 0
                            }
                        {
                            self.rc.code = self.rc.code << 8 | self.rc.read_byte()?;
                            self.rc.range <<= 8;
                            self.rc.low <<= 8;
                        }
                        self.found_state = s;
                        let sym = s.as_ref().symbol;
                        self.update1();
                        return Ok(sym as i32);
                    }
                    i = i.wrapping_sub(1);
                    if !(i != 0) {
                        break;
                    }
                }
                if hi_cnt >= summ_freq {
                    return Ok(SYM_ERROR);
                }
                hi_cnt = hi_cnt.wrapping_sub(count);
                self.rc.decode(hi_cnt, summ_freq.wrapping_sub(hi_cnt));

                char_mask = [u8::MAX; 256];

                let s2 = self.get_multi_state_stats(self.min_context);
                Self::mask_symbols(&mut char_mask, s, s2);
            } else {
                let mut s = self.get_single_state(self.min_context);
                let range = self.rc.range;
                let code = self.rc.code;
                let prob = self.get_bin_summ();

                let mut pr = *prob as u32;
                let size0 = (range >> 14) * pr;
                pr = ppmd_update_prob_1(pr);

                if code < size0 {
                    *prob = pr.wrapping_add((1 << PPMD_INT_BITS) as u32) as u16;
                    self.rc.range = size0;
                    while self.rc.low ^ self.rc.low.wrapping_add(self.rc.range) < K_TOP_VALUE
                        || self.rc.range < K_BOT_VALUE && {
                            self.rc.range = 0u32.wrapping_sub(self.rc.low) & (K_BOT_VALUE - 1);
                            1 != 0
                        }
                    {
                        self.rc.code = self.rc.code << 8 | self.rc.read_byte()?;
                        self.rc.range <<= 8;
                        self.rc.low <<= 8;
                    }
                    let freq = s.as_ref().freq as u32;
                    let c = self.get_context(s.as_ref().successor);
                    let sym = s.as_ref().symbol;
                    self.found_state = s;
                    self.prev_success = 1;
                    self.run_length += 1;
                    s.as_mut().freq = freq.wrapping_add((freq < 196) as u32) as u8;
                    if self.order_fall == 0 && c.addr() >= self.units_start.addr() {
                        self.min_context = c;
                        self.max_context = self.min_context;
                    } else {
                        self.update_model();
                    }
                    return Ok(sym as i32);
                }
                *prob = pr as u16;
                self.init_esc = self.exp_escape[(pr >> 10) as usize] as u32;
                self.rc.low = self.rc.low.wrapping_add(size0);
                self.rc.code = self.rc.code.wrapping_sub(size0);
                self.rc.range =
                    (self.rc.range & !((1 << (7 + 7)) as u32).wrapping_sub(1)).wrapping_sub(size0);

                char_mask = [u8::MAX; 256];

                *char_mask
                    .as_mut_ptr()
                    .offset(self.min_context.as_ref().data.single_state.symbol as isize) = 0;
                self.prev_success = 0;
            }
            loop {
                let mut freq_sum = 0;
                while self.rc.low ^ self.rc.low.wrapping_add(self.rc.range) < K_TOP_VALUE
                    || self.rc.range < K_BOT_VALUE && {
                        self.rc.range = 0u32.wrapping_sub(self.rc.low) & (K_BOT_VALUE - 1);
                        1 != 0
                    }
                {
                    self.rc.code = self.rc.code << 8 | self.rc.read_byte()?;
                    self.rc.range <<= 8;
                    self.rc.low <<= 8;
                }
                let mut mc = self.min_context;
                let num_masked = mc.as_ref().num_stats as u32;
                loop {
                    self.order_fall = self.order_fall.wrapping_add(1);
                    if mc.as_ref().suffix == 0 {
                        return Ok(SYM_END);
                    }
                    mc = self.get_context(mc.as_ref().suffix);
                    if !(mc.as_ref().num_stats as u32 == num_masked) {
                        break;
                    }
                }
                let s = self.get_multi_state_stats(mc);
                let mut num = (mc.as_ref().num_stats as u32).wrapping_add(1);
                let mut num2 = num.wrapping_div(2);
                num &= 1;
                let mut hi_cnt = s.as_ref().freq as u32
                    & *char_mask.as_mut_ptr().offset(s.as_ref().symbol as isize) as u32
                    & 0u32.wrapping_sub(num);
                let mut s = s.offset(num as isize);
                self.min_context = mc;
                loop {
                    let sym0 = s.offset(0).as_ref().symbol as u32;
                    let sym1 = s.offset(1).as_ref().symbol as u32;
                    s = s.offset(2);
                    hi_cnt = hi_cnt.wrapping_add(
                        s.offset(-2).as_ref().freq as u32
                            & *char_mask.as_mut_ptr().offset(sym0 as isize) as u32,
                    );
                    hi_cnt = hi_cnt.wrapping_add(
                        s.offset(-1).as_ref().freq as u32
                            & *char_mask.as_mut_ptr().offset(sym1 as isize) as u32,
                    );
                    num2 = num2.wrapping_sub(1);
                    if !(num2 != 0) {
                        break;
                    }
                }
                let see_source = self.make_esc_freq(num_masked, &mut freq_sum);
                freq_sum = freq_sum.wrapping_add(hi_cnt);
                let mut freq_sum2 = freq_sum;
                if freq_sum2 > self.rc.range {
                    freq_sum2 = self.rc.range;
                }
                self.rc.range /= freq_sum2;
                let mut count = self.rc.code / self.rc.range;
                if count < hi_cnt {
                    s = self.get_multi_state_stats(self.min_context);
                    hi_cnt = count;
                    loop {
                        count = count.wrapping_sub(
                            s.as_ref().freq as u32
                                & *(char_mask.as_mut_ptr()).offset(s.as_ref().symbol as isize)
                                    as u32,
                        );
                        s = s.offset(1);
                        if (count as i32) < 0 as i32 {
                            break;
                        }
                    }
                    s = s.offset(-1);
                    self.rc.decode(
                        hi_cnt
                            .wrapping_sub(count)
                            .wrapping_sub(s.as_ref().freq as u32),
                        s.as_ref().freq as u32,
                    );
                    while self.rc.low ^ self.rc.low.wrapping_add(self.rc.range) < K_TOP_VALUE
                        || self.rc.range < K_BOT_VALUE && {
                            self.rc.range = 0u32.wrapping_sub(self.rc.low) & (K_BOT_VALUE - 1);
                            1 != 0
                        }
                    {
                        self.rc.code = self.rc.code << 8 | self.rc.read_byte()?;
                        self.rc.range <<= 8;
                        self.rc.low <<= 8;
                    }

                    let see = self.get_see(see_source);
                    if (see.shift as i32) < 7 && {
                        see.count = see.count.wrapping_sub(1);
                        see.count as i32 == 0
                    } {
                        see.summ = ((see.summ as i32) << 1) as u16;
                        let fresh0 = see.shift;
                        see.shift = see.shift.wrapping_add(1);
                        see.count = (3 << fresh0 as i32) as u8;
                    }
                    self.found_state = s;
                    let sym = s.as_ref().symbol;
                    self.update2();
                    return Ok(sym as i32);
                }
                if count >= freq_sum2 {
                    return Ok(SYM_ERROR);
                }
                self.rc.decode(hi_cnt, freq_sum2.wrapping_sub(hi_cnt));
                let see = self.get_see(see_source);
                see.summ = (see.summ as u32).wrapping_add(freq_sum) as u16;

                s = self.get_multi_state_stats(self.min_context);
                let s2 = s
                    .offset(self.min_context.as_ref().num_stats as i32 as isize)
                    .offset(1);
                while s.addr() < s2.addr() {
                    char_mask[s.as_ref().symbol as usize] = 0;
                    s = s.offset(1);
                }
            }
        }
    }
}

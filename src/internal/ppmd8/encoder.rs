use super::*;
use crate::internal::ppmd_update_prob_1;

impl<W: Write> Ppmd8<RangeEncoder<W>> {
    pub(crate) fn encode_symbol(&mut self, symbol: u8) -> Result<(), std::io::Error> {
        unsafe {
            let mut char_mask: [u8; 256];
            if self.min_context.as_ref().num_stats != 0 {
                let mut s = self.get_multi_state_stats(self.min_context);
                let mut summ_freq = self.min_context.as_ref().data.multi_state.summ_freq as u32;
                summ_freq = self.rc.correct_sum_range(summ_freq);

                if s.as_ref().symbol == symbol {
                    self.rc.encode(0, s.as_ref().freq as u32, summ_freq);
                    while self.rc.low ^ self.rc.low.wrapping_add(self.rc.range) < K_TOP_VALUE
                        || self.rc.range < K_BOT_VALUE && {
                            self.rc.range = 0u32.wrapping_sub(self.rc.low) & (K_BOT_VALUE - 1);
                            1 != 0
                        }
                    {
                        self.rc.write_byte((self.rc.low >> 24) as u8)?;
                        self.rc.range <<= 8;
                        self.rc.low <<= 8;
                    }
                    self.found_state = s;
                    self.update1_0();
                    return Ok(());
                }
                self.prev_success = 0;
                let mut sum = s.as_ref().freq as u32;
                let num_stats = self.min_context.as_ref().num_stats as u32;
                for _ in 0..num_stats {
                    s = s.offset(1);
                    if s.as_ref().symbol == symbol {
                        self.rc.encode(sum, s.as_ref().freq as u32, summ_freq);
                        while self.rc.low ^ self.rc.low.wrapping_add(self.rc.range) < K_TOP_VALUE
                            || self.rc.range < K_BOT_VALUE && {
                                self.rc.range = 0u32.wrapping_sub(self.rc.low) & (K_BOT_VALUE - 1);
                                1 != 0
                            }
                        {
                            self.rc.write_byte((self.rc.low >> 24) as u8)?;
                            self.rc.range <<= 8;
                            self.rc.low <<= 8;
                        }
                        self.found_state = s;
                        self.update1();
                        return Ok(());
                    }
                    sum = sum.wrapping_add(s.as_ref().freq as u32);
                }
                self.rc.encode(sum, summ_freq.wrapping_sub(sum), summ_freq);

                char_mask = [u8::MAX; 256];

                let s2 = self.get_multi_state_stats(self.min_context);
                Self::mask_symbols(&mut char_mask, s, s2);
            } else {
                let mut s = self.get_single_state(self.min_context);
                let range = self.rc.range;
                let prob = self.get_bin_summ();

                let mut pr = *prob as u32;
                let bound = (range >> 14) * pr;
                pr = ppmd_update_prob_1(pr);

                if s.as_ref().symbol == symbol {
                    *prob = pr.wrapping_add((1 << 7) as u32) as u16;
                    self.rc.range = bound;
                    while self.rc.low ^ self.rc.low.wrapping_add(self.rc.range) < K_TOP_VALUE
                        || self.rc.range < K_BOT_VALUE && {
                            self.rc.range = 0u32.wrapping_sub(self.rc.low) & (K_BOT_VALUE - 1);
                            1 != 0
                        }
                    {
                        self.rc.write_byte((self.rc.low >> 24) as u8)?;
                        self.rc.range <<= 8;
                        self.rc.low <<= 8;
                    }
                    let freq = s.as_ref().freq as u32;
                    let c = self.get_context(s.as_ref().successor);
                    self.found_state = s;
                    self.prev_success = 1;
                    self.run_length += 1;
                    s.as_mut().freq = (freq + ((freq < 196) as u32)) as u8;
                    if self.order_fall == 0 && c.addr() >= self.units_start.addr() {
                        self.min_context = c;
                        self.max_context = self.min_context;
                    } else {
                        self.update_model();
                    }
                    return Ok(());
                }
                *prob = pr as u16;
                self.init_esc = self.exp_escape[(pr >> 10) as usize] as u32;
                self.rc.low += bound;
                self.rc.range =
                    (self.rc.range & !((1 << (7 + 7)) as u32).wrapping_sub(1)).wrapping_sub(bound);

                char_mask = [u8::MAX; 256];
                char_mask[s.as_ref().symbol as usize] = 0;
                self.prev_success = 0;
            }
            loop {
                let mut esc_freq = 0;
                while self.rc.low ^ self.rc.low.wrapping_add(self.rc.range) < K_TOP_VALUE
                    || self.rc.range < K_BOT_VALUE && {
                        self.rc.range = 0u32.wrapping_sub(self.rc.low) & (K_BOT_VALUE - 1);
                        1 != 0
                    }
                {
                    self.rc.write_byte((self.rc.low >> 24) as u8)?;
                    self.rc.range <<= 8;
                    self.rc.low <<= 8;
                }
                let mut mc = self.min_context;
                let num_masked = mc.as_ref().num_stats as u32;
                while mc.as_ref().num_stats as u32 == num_masked {
                    self.order_fall += 1;
                    if mc.as_ref().suffix == 0 {
                        return Ok(());
                    }
                    mc = self.get_context(mc.as_ref().suffix);
                }
                self.min_context = mc;
                let see_source = self.make_esc_freq(num_masked, &mut esc_freq);
                let mut s = self.get_multi_state_stats(self.min_context);
                let mut sum = 0u32;
                let mut i = (self.min_context.as_ref().num_stats as u32) + 1;
                while i != 0 {
                    let cur = s.as_ref().symbol as u32;
                    if cur as i32 == symbol as i32 {
                        let low = sum;
                        let freq = s.as_ref().freq as u32;

                        let see = self.get_see(see_source);
                        if (see.shift as i32) < 7 && {
                            see.count -= 1;
                            see.count as i32 == 0
                        } {
                            see.summ = ((see.summ as i32) << 1) as u16;
                            let fresh = see.shift as u32;
                            see.shift += 1;
                            see.count = (3 << fresh) as u8;
                        }
                        self.found_state = s;
                        sum += esc_freq;
                        let num2 = i / 2;
                        i &= 1;
                        sum += freq & 0u32.wrapping_sub(i);
                        if num2 != 0 {
                            s = s.offset(i as isize);
                            for _ in 0..num2 {
                                let sym0 = s.offset(0).as_ref().symbol as u32;
                                let sym1 = s.offset(1).as_ref().symbol as u32;
                                s = s.offset(2);
                                sum += s.offset(-2).as_ref().freq as u32
                                    & char_mask[sym0 as usize] as u32;
                                sum += s.offset(-1).as_ref().freq as u32
                                    & char_mask[sym1 as usize] as u32;
                            }
                        }

                        sum = self.rc.correct_sum_range(sum);

                        self.rc.encode(low, freq, sum);
                        while self.rc.low ^ self.rc.low.wrapping_add(self.rc.range) < K_TOP_VALUE
                            || self.rc.range < K_BOT_VALUE && {
                                self.rc.range = 0u32.wrapping_sub(self.rc.low) & (K_BOT_VALUE - 1);
                                1 != 0
                            }
                        {
                            self.rc.write_byte((self.rc.low >> 24) as u8)?;

                            self.rc.range <<= 8;
                            self.rc.low <<= 8;
                        }
                        self.update2();
                        return Ok(());
                    }
                    sum += s.as_ref().freq as u32 & char_mask[cur as usize] as u32;
                    s = s.offset(1);
                    i = i.wrapping_sub(1);
                }
                let mut total = sum.wrapping_add(esc_freq);
                let see = self.get_see(see_source);
                see.summ = (see.summ as u32).wrapping_add(total) as u16;

                total = self.rc.correct_sum_range(total);

                self.rc.encode(sum, total.wrapping_sub(sum), total);

                let s2 = self.get_multi_state_stats(self.min_context);
                s = s.offset(-1);
                Self::mask_symbols(&mut char_mask, s, s2);
            }
        }
    }
}

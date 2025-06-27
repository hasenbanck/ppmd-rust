use super::*;
use crate::internal::{ppmd_update_prob_1, PPMD_INT_BITS};

impl<W: Write> Ppmd8<RangeEncoder<W>> {
    pub(crate) fn encode_symbol(&mut self, symbol: u8) -> Result<(), std::io::Error> {
        unsafe {
            let mut char_mask: [u8; 256];
            if self.min_context.as_ref().num_stats != 0 {
                let mut s = self.get_multi_state_stats(self.min_context);
                let mut summ_freq = self.min_context.as_ref().data.multi_state.summ_freq as u32;

                summ_freq = self.rc.correct_sum_range(summ_freq);

                if s.as_ref().symbol == symbol {
                    self.rc.encode_final(0, s.as_ref().freq as u32, summ_freq)?;
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
                        self.rc
                            .encode_final(sum, s.as_ref().freq as u32, summ_freq)?;
                        self.found_state = s;
                        self.update1();
                        return Ok(());
                    }
                    sum += s.as_ref().freq as u32;
                }

                self.rc.encode(sum, summ_freq.wrapping_sub(sum), summ_freq);

                char_mask = [u8::MAX; 256];

                let s2 = self.get_multi_state_stats(self.min_context);
                Self::mask_symbols(&mut char_mask, s, s2);
            } else {
                let s = self.get_single_state(self.min_context);
                let range = self.rc.range;
                let prob = self.get_bin_summ();

                let mut pr = *prob as u32;
                let bound = (range >> 14) * pr;
                pr = ppmd_update_prob_1(pr);

                if s.as_ref().symbol == symbol {
                    *prob = (pr + (1 << PPMD_INT_BITS) as u32) as u16;
                    self.rc.range = bound;
                    self.rc.normalize_remote()?;

                    self.update_bin(s);
                    return Ok(());
                }

                *prob = pr as u16;
                self.init_esc = self.exp_escape[(pr >> 10) as usize] as u32;

                self.rc.encode_bit_1(bound);

                char_mask = [u8::MAX; 256];
                char_mask[s.as_ref().symbol as usize] = 0;
                self.prev_success = 0;
            }
            loop {
                let mut esc_freq = 0;

                self.rc.normalize_remote()?;

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
                        see.update();
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

                        self.rc.encode_final(low, freq, sum)?;
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

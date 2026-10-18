use super::*;

impl<RC: Ppmd7RangeEncoder> PPMd7<RC> {
    pub(crate) fn encode_symbol(&mut self, symbol: i32) -> Result<(), std::io::Error> {
        let mut char_mask: [u8; 256];

        if self.mem.num_stats(self.min_context) != 1 {
            let mut s = self.mem.stats(self.min_context);

            self.rc.div_range(self.mem.summ_freq(self.min_context));

            if self.mem.symbol(s) as i32 == symbol {
                self.rc.encode_final(0, self.mem.freq(s) as u32)?;
                self.found_state = s;
                self.update1_0();
                return Ok(());
            }
            self.prev_success = 0;

            let mut sum = self.mem.freq(s) as u32;

            for _ in 1..self.mem.num_stats(self.min_context) {
                s = s.next();
                if self.mem.symbol(s) as i32 == symbol {
                    self.rc.encode_final(sum, self.mem.freq(s) as u32)?;
                    self.found_state = s;
                    self.update1();
                    return Ok(());
                }
                sum += self.mem.freq(s) as u32;
            }

            self.rc
                .encode(sum, self.mem.summ_freq(self.min_context) - sum);

            self.hi_bits_flag = Self::hi_bits_flag3(self.mem.symbol(self.found_state) as u32);
            char_mask = [u8::MAX; 256];

            let s2 = self.mem.stats(self.min_context);
            self.mask_symbols(&mut char_mask, s, s2);
        } else {
            let s = self.min_context.one_state();
            let s_symbol = self.mem.symbol(s);
            let range = self.rc.range();
            let prob = self.get_bin_summ();

            let mut pr = *prob as u32;
            let bound = (range >> 14) * pr;
            pr = ppmd_update_prob_1(pr);

            if s_symbol as i32 == symbol {
                *prob = (pr + (1 << PPMD_INT_BITS)) as u16;

                self.rc.encode_bit_0(bound)?;
                self.update_bin(s);

                return Ok(());
            }
            *prob = pr as u16;
            self.init_esc = self.exp_escape[(pr >> 10) as usize] as u32;
            self.rc.encode_bit_1(bound)?;

            char_mask = [u8::MAX; 256];
            char_mask[s_symbol as usize] = 0;
            self.prev_success = 0;
        }
        loop {
            self.rc.normalize_remote()?;

            let mut mc = self.min_context;
            let num_masked = self.mem.num_stats(mc);

            let mut i;

            loop {
                self.order_fall += 1;
                if self.mem.suffix(mc).is_null() {
                    return Ok(());
                }
                mc = self.mem.suffix(mc);
                i = self.mem.num_stats(mc);

                if i != num_masked {
                    break;
                }
            }

            self.min_context = mc;

            let mut esc_freq = 0;
            let mut s = self.mem.stats(mc);
            let see_source = self.make_esc_freq(num_masked, &mut esc_freq);
            let mut sum = 0;

            while i != 0 {
                let cur = self.mem.symbol(s);
                if cur as i32 == symbol {
                    let low = sum;
                    let freq = self.mem.freq(s) as u32;

                    let see = self.get_see(see_source);
                    see.update();
                    self.found_state = s;
                    sum += esc_freq;

                    let mut num2 = i / 2;
                    i &= 1;
                    sum += freq & 0u32.wrapping_sub(i);
                    if num2 != 0 {
                        s = s.add(i);

                        while num2 != 0 {
                            let sym0 = self.mem.symbol(s);
                            let sym1 = self.mem.symbol(s.next());
                            sum += (self.mem.freq(s) & char_mask[sym0 as usize]) as u32;
                            sum += (self.mem.freq(s.next()) & char_mask[sym1 as usize]) as u32;
                            s = s.add(2);
                            num2 -= 1;
                        }
                    }
                    self.rc.div_range(sum);
                    self.rc.encode_final(low, freq)?;
                    self.update2();
                    return Ok(());
                }
                sum += self.mem.freq(s) as u32 & char_mask[cur as usize] as u32;
                s = s.next();
                i -= 1;
            }

            {
                let total = sum + esc_freq;
                let see = self.get_see(see_source);
                see.summ = ((see.summ as u32) + total) as u16;

                self.rc.div_range(total);
                self.rc.encode(sum, esc_freq);
            }

            let s2 = self.mem.stats(self.min_context);
            s = s.prev();
            self.mask_symbols(&mut char_mask, s, s2);
        }
    }
}

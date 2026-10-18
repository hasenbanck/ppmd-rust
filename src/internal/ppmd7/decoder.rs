use super::*;
use crate::{SYM_END, SYM_ERROR};

impl<RC: Ppmd7RangeDecoder> PPMd7<RC> {
    pub(crate) fn decode_symbol(&mut self) -> Result<i32, std::io::Error> {
        let mut char_mask: [u8; 256];

        if self.mem.num_stats(self.min_context) != 1 {
            let mut s = self.mem.stats(self.min_context);
            let summ_freq = self.mem.summ_freq(self.min_context);

            let mut count = self.rc.get_threshold(summ_freq);
            let hi_cnt = count;

            let freq = self.mem.freq(s) as u32;
            count = count.wrapping_sub(freq);
            if (count as i32) < 0 {
                self.rc.decode_final(0, freq)?;
                self.found_state = s;
                let sym = self.mem.symbol(s);
                self.update1_0();
                return Ok(sym as i32);
            }

            self.prev_success = 0;

            let num_stats = self.mem.num_stats(self.min_context);
            for _ in 1..num_stats {
                s = s.next();
                let freq = self.mem.freq(s) as u32;
                count = count.wrapping_sub(freq);
                if (count as i32) < 0 {
                    self.rc
                        .decode_final(hi_cnt.wrapping_sub(count).wrapping_sub(freq), freq)?;
                    self.found_state = s;
                    let sym = self.mem.symbol(s);
                    self.update1();
                    return Ok(sym as i32);
                }
            }

            if hi_cnt >= summ_freq {
                return Ok(SYM_ERROR);
            }

            let hi_cnt = hi_cnt.wrapping_sub(count);
            self.rc.decode(hi_cnt, summ_freq.wrapping_sub(hi_cnt));

            self.hi_bits_flag = Self::hi_bits_flag3(self.mem.symbol(self.found_state) as u32);
            char_mask = [u8::MAX; 256];

            let s2 = self.mem.stats(self.min_context);
            self.mask_symbols(&mut char_mask, s, s2);
        } else {
            let s = self.min_context.one_state();
            let range = self.rc.range();
            let code = self.rc.code();
            let prob = self.get_bin_summ();

            let mut pr: u32 = *prob as u32;
            let size0: u32 = (range >> 14) * pr;
            pr = ppmd_update_prob_1(pr);

            if code < size0 {
                *prob = (pr + (1 << PPMD_INT_BITS)) as u16;

                self.rc.decode_bit_0(size0)?;

                let sym = self.mem.symbol(s);
                self.update_bin(s);
                return Ok(sym as i32);
            }

            *prob = pr as u16;
            self.init_esc = self.exp_escape[(pr >> 10) as usize] as u32;

            self.rc.decode_bit_1(size0);

            char_mask = [u8::MAX; 256];
            char_mask[self.mem.symbol(s) as usize] = 0;
            self.prev_success = 0;
        }
        loop {
            self.rc.normalize_remote()?;
            let mut mc = self.min_context;
            let num_masked = self.mem.num_stats(mc);

            while self.mem.num_stats(mc) == num_masked {
                self.order_fall += 1;
                if self.mem.suffix(mc).is_null() {
                    return Ok(SYM_END);
                }
                mc = self.mem.suffix(mc);
            }

            let mut s = self.mem.stats(mc);

            let mut num = self.mem.num_stats(mc);
            let mut num2 = num / 2;

            num &= 1;
            let mut hi_cnt = self.mem.freq(s) as u32
                & char_mask[self.mem.symbol(s) as usize] as u32
                & (0u32.wrapping_sub(num));
            s = s.add(num);
            self.min_context = mc;

            while num2 != 0 {
                let sym0 = self.mem.symbol(s);
                let sym1 = self.mem.symbol(s.next());
                hi_cnt += (self.mem.freq(s) & char_mask[sym0 as usize]) as u32;
                hi_cnt += (self.mem.freq(s.next()) & char_mask[sym1 as usize]) as u32;
                s = s.add(2);
                num2 -= 1;
            }

            let mut freq_sum = 0;
            let see_source = self.make_esc_freq(num_masked, &mut freq_sum);
            freq_sum += hi_cnt;

            let mut count = self.rc.get_threshold(freq_sum);

            if count < hi_cnt {
                s = self.mem.stats(self.min_context);
                hi_cnt = count;
                loop {
                    count = count.wrapping_sub(
                        self.mem.freq(s) as u32 & char_mask[self.mem.symbol(s) as usize] as u32,
                    );
                    s = s.next();
                    if (count as i32) < 0 {
                        break;
                    }
                }
                s = s.prev();

                let freq = self.mem.freq(s) as u32;
                self.rc
                    .decode_final(hi_cnt.wrapping_sub(count).wrapping_sub(freq), freq)?;

                let see = self.get_see(see_source);
                see.update();

                self.found_state = s;
                let sym = self.mem.symbol(s);
                self.update2();
                return Ok(sym as i32);
            }

            if count >= freq_sum {
                return Ok(SYM_ERROR);
            }

            self.rc.decode(hi_cnt, freq_sum - hi_cnt);

            // We increase see.summ for sum of freqs of all non_masked symbols.
            // New see.summ value can overflow over 16-bits in some rare cases.
            let see = self.get_see(see_source);
            see.summ = see.summ.wrapping_add(freq_sum as u16);

            s = self.mem.stats(self.min_context);
            let s2 = s.add(self.mem.num_stats(self.min_context));
            while s < s2 {
                char_mask[self.mem.symbol(s) as usize] = 0;
                s = s.next();
            }
        }
    }
}

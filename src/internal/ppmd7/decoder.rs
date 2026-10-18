use super::*;
use crate::{SYM_END, SYM_ERROR};

impl<RC: Ppmd7RangeDecoder> PPMd7<RC> {
    pub(crate) fn decode_symbol(&mut self) -> Result<i32, std::io::Error> {
        let mut char_mask: [u8; 256];

        let num_stats = self.mem.num_stats(self.min_context);
        if num_stats != 1 {
            let stats = self.mem.stats(self.min_context);
            let summ_freq = self.mem.summ_freq(self.min_context);

            let mut count = self.rc.get_threshold(summ_freq);
            let hi_cnt = count;

            let states = self.mem.states(stats, num_stats);
            let freq = states[1] as u32;
            count = count.wrapping_sub(freq);
            if (count as i32) < 0 {
                self.rc.decode_final(0, freq)?;
                self.found_state = stats;
                let sym = states[0];
                self.update1_0();
                return Ok(sym as i32);
            }

            self.prev_success = 0;

            let mut s = stats;
            let mut rest = &states[STATE_SIZE as usize..];
            while let [sym, freq, _, _, _, _, tail @ ..] = rest {
                s = s.next();
                let freq = *freq as u32;
                count = count.wrapping_sub(freq);
                if (count as i32) < 0 {
                    self.rc
                        .decode_final(hi_cnt.wrapping_sub(count).wrapping_sub(freq), freq)?;
                    self.found_state = s;
                    let sym = *sym;
                    self.update1();
                    return Ok(sym as i32);
                }
                rest = tail;
            }

            if hi_cnt >= summ_freq {
                return Ok(SYM_ERROR);
            }

            let hi_cnt = hi_cnt.wrapping_sub(count);
            self.rc.decode(hi_cnt, summ_freq.wrapping_sub(hi_cnt));

            self.hi_bits_flag = Self::hi_bits_flag3(self.mem.symbol(self.found_state) as u32);
            char_mask = [u8::MAX; 256];

            self.mask_symbols(&mut char_mask, stats, num_stats);
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

            let stats = self.mem.stats(mc);
            let num_stats = self.mem.num_stats(mc);
            self.min_context = mc;

            let mut hi_cnt = 0;
            for state in self
                .mem
                .states(stats, num_stats)
                .chunks_exact(STATE_SIZE as usize)
            {
                hi_cnt += (state[1] & char_mask[state[0] as usize]) as u32;
            }

            let mut freq_sum = 0;
            let see_source = self.make_esc_freq(num_masked, &mut freq_sum);
            freq_sum += hi_cnt;

            let mut count = self.rc.get_threshold(freq_sum);

            if count < hi_cnt {
                hi_cnt = count;
                let mut s = stats;
                let mut freq = 0;
                let mut rest = self.mem.states(stats, num_stats);
                while let [sym, f, _, _, _, _, tail @ ..] = rest {
                    freq = (f & char_mask[*sym as usize]) as u32;
                    count = count.wrapping_sub(freq);
                    if (count as i32) < 0 {
                        break;
                    }
                    s = s.next();
                    rest = tail;
                }

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

            self.mask_symbols(&mut char_mask, stats, num_stats);
        }
    }
}

use super::*;
use crate::{SYM_END, SYM_ERROR};

impl<R: Read> PPMd8<RangeDecoder<R>> {
    pub(crate) fn decode_symbol(&mut self) -> Result<i32, std::io::Error> {
        let mut char_mask: [u8; 256];

        let num_stats = self.mem.num_stats(self.min_context);
        if num_stats != 0 {
            let stats = self.mem.stats(self.min_context);
            let mut summ_freq = self.mem.summ_freq(self.min_context);

            summ_freq = self.rc.correct_sum_range(summ_freq);

            let mut count = self.rc.get_threshold(summ_freq);
            let mut hi_cnt = count;

            let states = self.mem.states(stats, num_stats + 1);
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

            hi_cnt = hi_cnt.wrapping_sub(count);
            self.rc.decode(hi_cnt, summ_freq.wrapping_sub(hi_cnt));

            char_mask = [u8::MAX; 256];

            self.mask_symbols(&mut char_mask, stats, num_stats + 1);
        } else {
            let s = self.min_context.one_state();
            let range = self.rc.range;
            let code = self.rc.code;
            let prob = self.get_bin_summ();

            let mut pr = *prob as u32;
            let size0 = (range >> 14) * pr;
            pr = ppmd_update_prob_1(pr);

            if code < size0 {
                *prob = pr.wrapping_add(1 << PPMD_INT_BITS) as u16;
                self.rc.range = size0;
                self.rc.normalize_remote()?;

                let sym = self.update_bin(s);
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
            let mut freq_sum = 0;
            self.rc.normalize_remote()?;
            let mut mc = self.min_context;
            let num_masked = self.mem.num_stats(mc);

            loop {
                self.order_fall += 1;
                if self.mem.suffix(mc).is_null() {
                    return Ok(SYM_END);
                }
                mc = self.mem.suffix(mc);

                if self.mem.num_stats(mc) != num_masked {
                    break;
                }
            }

            let stats = self.mem.stats(mc);
            let num_stats = self.mem.num_stats(mc) + 1;
            self.min_context = mc;

            let mut hi_cnt = 0;
            for state in self
                .mem
                .states(stats, num_stats)
                .chunks_exact(STATE_SIZE as usize)
            {
                hi_cnt += (state[1] & char_mask[state[0] as usize]) as u32;
            }

            let see_source = self.make_esc_freq(num_masked, &mut freq_sum);
            freq_sum += hi_cnt;
            let freq_sum2 = self.rc.correct_sum_range(freq_sum);

            let mut count = self.rc.get_threshold(freq_sum2);

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

            if count >= freq_sum2 {
                return Ok(SYM_ERROR);
            }

            self.rc.decode(hi_cnt, freq_sum2.wrapping_sub(hi_cnt));

            // We increase (see.summ) for sum of frequencies of all non_masked symbols.
            // new (see.summ) value can overflow over 16-bits in some rare cases.
            let see = self.get_see(see_source);
            see.summ = (see.summ as u32).wrapping_add(freq_sum) as u16;

            self.mask_symbols(&mut char_mask, stats, num_stats);
        }
    }
}

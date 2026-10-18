use super::*;

impl<W: Write> PPMd8<RangeEncoder<W>> {
    pub(crate) fn encode_symbol(&mut self, symbol: i32) -> std::io::Result<()> {
        let mut char_mask: [u8; 256];

        let num_stats = self.mem.num_stats(self.min_context);
        if num_stats != 0 {
            let stats = self.mem.stats(self.min_context);
            let mut summ_freq = self.mem.summ_freq(self.min_context);

            summ_freq = self.rc.correct_sum_range(summ_freq);

            let states = self.mem.states(stats, num_stats + 1);
            if states[0] as i32 == symbol {
                self.rc.encode_final(0, states[1] as u32, summ_freq)?;
                self.found_state = stats;
                self.update1_0();
                return Ok(());
            }

            self.prev_success = 0;
            let mut sum = states[1] as u32;

            let mut s = stats;
            for state in states[STATE_SIZE as usize..].chunks_exact(STATE_SIZE as usize) {
                s = s.next();
                if state[0] as i32 == symbol {
                    self.rc.encode_final(sum, state[1] as u32, summ_freq)?;
                    self.found_state = s;
                    self.update1();
                    return Ok(());
                }
                sum += state[1] as u32;
            }

            self.rc.encode(sum, summ_freq.wrapping_sub(sum), summ_freq);

            char_mask = [u8::MAX; 256];

            self.mask_symbols(&mut char_mask, stats, num_stats + 1);
        } else {
            let s = self.min_context.one_state();
            let s_symbol = self.mem.symbol(s);
            let range = self.rc.range;
            let prob = self.get_bin_summ();

            let mut pr = *prob as u32;
            let bound = (range >> 14) * pr;
            pr = ppmd_update_prob_1(pr);

            if s_symbol as i32 == symbol {
                *prob = (pr + (1 << PPMD_INT_BITS)) as u16;
                self.rc.range = bound;
                self.rc.normalize_remote()?;

                self.update_bin(s);
                return Ok(());
            }

            *prob = pr as u16;
            self.init_esc = self.exp_escape[(pr >> 10) as usize] as u32;

            self.rc.encode_bit_1(bound);

            char_mask = [u8::MAX; 256];
            char_mask[s_symbol as usize] = 0;
            self.prev_success = 0;
        }
        loop {
            let mut esc_freq = 0;

            self.rc.normalize_remote()?;

            let mut mc = self.min_context;
            let num_masked = self.mem.num_stats(mc);

            loop {
                self.order_fall += 1;
                if self.mem.suffix(mc).is_null() {
                    // EndMarker (symbol = -1)
                    return Ok(());
                }
                mc = self.mem.suffix(mc);

                if self.mem.num_stats(mc) != num_masked {
                    break;
                }
            }

            self.min_context = mc;

            let see_source = self.make_esc_freq(num_masked, &mut esc_freq);

            let stats = self.mem.stats(self.min_context);
            let num_stats = self.mem.num_stats(self.min_context) + 1;
            let mut sum = 0u32;

            let mut s = stats;
            let mut states = self
                .mem
                .states(stats, num_stats)
                .chunks_exact(STATE_SIZE as usize);
            while let Some(state) = states.next() {
                if state[0] as i32 == symbol {
                    let low = sum;
                    let freq = state[1] as u32;

                    // The total adds the escape frequency and the unmasked frequencies of this
                    // state and of all states after it.
                    sum += esc_freq + freq;
                    for state in states {
                        sum += (state[1] & char_mask[state[0] as usize]) as u32;
                    }

                    let see = self.get_see(see_source);
                    see.update();
                    self.found_state = s;

                    sum = self.rc.correct_sum_range(sum);

                    self.rc.encode_final(low, freq, sum)?;
                    self.update2();
                    return Ok(());
                }
                sum += (state[1] & char_mask[state[0] as usize]) as u32;
                s = s.next();
            }

            let mut total = sum.wrapping_add(esc_freq);
            let see = self.get_see(see_source);
            see.summ = (see.summ as u32).wrapping_add(total) as u16;

            total = self.rc.correct_sum_range(total);

            self.rc.encode(sum, total.wrapping_sub(sum), total);

            self.mask_symbols(&mut char_mask, stats, num_stats);
        }
    }
}

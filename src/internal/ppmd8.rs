mod decoder;
mod encoder;
mod range_coding;

use std::io::{Read, Write};

pub(crate) use range_coding::{RangeDecoder, RangeEncoder};

use super::*;
use crate::{Error, RestoreMethod, PPMD8_MAX_ORDER};

const MAX_FREQ: u8 = 124;
const K_TOP_VALUE: u32 = 1 << 24;
const K_BOT_VALUE: u32 = 1 << 15;
const EMPTY_NODE: u32 = u32::MAX;
const FLAG_RESCALED: u8 = 1 << 2;
const FLAG_PREV_HIGH: u8 = 1 << 4;

static K_EXP_ESCAPE: [u8; 16] = [25, 14, 9, 7, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 2, 2];

static K_INIT_BIN_ESC: [u16; 8] = [
    0x3CDD, 0x1F3F, 0x59BF, 0x48F3, 0x64A1, 0x5ABC, 0x6632, 0x6051,
];

/// The context and free node records of PPMd8.
///
/// A context record has 12 bytes:
///
/// | Offset | Field                                                      |
/// |--------|------------------------------------------------------------|
/// | 0      | `num_stats: u8` (the number of states minus one)           |
/// | 1      | `flags: u8`                                                |
/// | 2      | `summ_freq: u16`, or the one state of a binary context     |
/// | 4      | `stats: u32`, or the successor of the one state            |
/// | 8      | `suffix: u32`                                              |
///
/// A free node record has 12 bytes: `stamp: u32`, `next: u32`, `nu: u32`.
trait Records {
    fn num_stats(&self, ctx: CtxRef) -> u32;
    fn set_num_stats(&mut self, ctx: CtxRef, value: u32);
    fn flags(&self, ctx: CtxRef) -> u8;
    fn set_flags(&mut self, ctx: CtxRef, value: u8);
    fn summ_freq(&self, ctx: CtxRef) -> u32;
    fn set_summ_freq(&mut self, ctx: CtxRef, value: u32);
    fn stats(&self, ctx: CtxRef) -> StateRef;
    fn set_stats(&mut self, ctx: CtxRef, value: StateRef);
    fn suffix(&self, ctx: CtxRef) -> CtxRef;
    fn set_suffix(&mut self, ctx: CtxRef, value: CtxRef);
    fn stamp(&self, node: UnitRef) -> u32;
    fn set_stamp(&mut self, node: UnitRef, value: u32);
    fn node_next(&self, node: UnitRef) -> UnitRef;
    fn set_node_next(&mut self, node: UnitRef, value: UnitRef);
    fn nu(&self, node: UnitRef) -> u32;
    fn set_nu(&mut self, node: UnitRef, value: u32);
}

impl Records for Memory {
    #[inline(always)]
    fn num_stats(&self, ctx: CtxRef) -> u32 {
        self.u8(ctx.offset()) as u32
    }

    #[inline(always)]
    fn set_num_stats(&mut self, ctx: CtxRef, value: u32) {
        self.set_u8(ctx.offset(), value as u8);
    }

    #[inline(always)]
    fn flags(&self, ctx: CtxRef) -> u8 {
        self.u8(ctx.offset() + 1)
    }

    #[inline(always)]
    fn set_flags(&mut self, ctx: CtxRef, value: u8) {
        self.set_u8(ctx.offset() + 1, value);
    }

    #[inline(always)]
    fn summ_freq(&self, ctx: CtxRef) -> u32 {
        self.u16(ctx.offset() + 2) as u32
    }

    #[inline(always)]
    fn set_summ_freq(&mut self, ctx: CtxRef, value: u32) {
        self.set_u16(ctx.offset() + 2, value as u16);
    }

    #[inline(always)]
    fn stats(&self, ctx: CtxRef) -> StateRef {
        StateRef::at(self.u32(ctx.offset() + 4))
    }

    #[inline(always)]
    fn set_stats(&mut self, ctx: CtxRef, value: StateRef) {
        self.set_u32(ctx.offset() + 4, value.offset());
    }

    #[inline(always)]
    fn suffix(&self, ctx: CtxRef) -> CtxRef {
        CtxRef::at(self.u32(ctx.offset() + 8))
    }

    #[inline(always)]
    fn set_suffix(&mut self, ctx: CtxRef, value: CtxRef) {
        self.set_u32(ctx.offset() + 8, value.offset());
    }

    #[inline(always)]
    fn stamp(&self, node: UnitRef) -> u32 {
        self.u32(node.offset())
    }

    #[inline(always)]
    fn set_stamp(&mut self, node: UnitRef, value: u32) {
        self.set_u32(node.offset(), value);
    }

    #[inline(always)]
    fn node_next(&self, node: UnitRef) -> UnitRef {
        UnitRef::at(self.u32(node.offset() + 4))
    }

    #[inline(always)]
    fn set_node_next(&mut self, node: UnitRef, value: UnitRef) {
        self.set_u32(node.offset() + 4, value.offset());
    }

    #[inline(always)]
    fn nu(&self, node: UnitRef) -> u32 {
        self.u32(node.offset() + 8)
    }

    #[inline(always)]
    fn set_nu(&mut self, node: UnitRef, value: u32) {
        self.set_u32(node.offset() + 8, value);
    }
}

pub(crate) struct PPMd8<RC> {
    min_context: CtxRef,
    max_context: CtxRef,
    found_state: StateRef,
    order_fall: u32,
    init_esc: u32,
    prev_success: u32,
    max_order: u32,
    restore_method: RestoreMethod,
    run_length: i32,
    init_rl: i32,
    size: u32,
    glue_count: u32,
    align_offset: u32,
    lo_unit: u32,
    hi_unit: u32,
    text: u32,
    units_start: u32,
    index2units: [u8; 40],
    units2index: [u8; 128],
    free_list: [UnitRef; 38],
    stamps: [u32; 38],
    ns2bs_index: [u8; 256],
    ns2index: [u8; 260],
    exp_escape: [u8; 16],
    dummy_see: See,
    see: [[See; 32]; 24],
    bin_summ: [[u16; 64]; 25],
    mem: Memory,
    rc: RC,
}

impl<RC> PPMd8<RC> {
    fn construct(
        rc: RC,
        max_order: u32,
        mem_size: u32,
        restore_method: RestoreMethod,
    ) -> crate::Result<Self> {
        let mut units2index = [0u8; 128];
        let mut index2units = [0u8; 40];

        let mut k = 0;
        for i in 0..PPMD_NUM_INDEXES {
            let mut step = if i >= 12 { 4 } else { (i >> 2) + 1 };
            loop {
                units2index[k as usize] = i as u8;
                k += 1;

                step -= 1;
                if step == 0 {
                    break;
                }
            }
            index2units[i as usize] = k as u8;
        }

        let mut ns2bs_index = [0u8; 256];
        ns2bs_index[0] = (0 << 1) as u8;
        ns2bs_index[1] = (1 << 1) as u8;
        ns2bs_index[2..11].fill((2 << 1) as u8);
        ns2bs_index[11..256].fill((3 << 1) as u8);

        let mut ns2index = [0u8; 260];
        for i in 0..5 {
            ns2index[i as usize] = i as u8;
        }

        let mut m = 5;
        let mut k = 1;
        for i in 5..260 {
            ns2index[i as usize] = m as u8;
            k -= 1;
            if k == 0 {
                m += 1;
                k = m - 4;
            }
        }

        let align_offset = (4u32.wrapping_sub(mem_size)) & 3;
        let total_size = (align_offset as usize)
            .checked_add(mem_size as usize)
            .ok_or(Error::MemoryAllocation)?;
        let mem = Memory::new(total_size)?;

        let mut ppmd = Self {
            min_context: CtxRef::NULL,
            max_context: CtxRef::NULL,
            found_state: StateRef::NULL,
            order_fall: 0,
            init_esc: 0,
            prev_success: 0,
            max_order,
            restore_method,
            run_length: 0,
            init_rl: 0,
            size: mem_size,
            glue_count: 0,
            align_offset,
            lo_unit: 0,
            hi_unit: 0,
            text: 0,
            units_start: 0,
            index2units,
            units2index,
            free_list: [UnitRef::NULL; 38],
            stamps: [0; 38],
            ns2bs_index,
            ns2index,
            exp_escape: K_EXP_ESCAPE,
            dummy_see: See::default(),
            see: [[See::default(); 32]; 24],
            bin_summ: [[0; 64]; 25],
            mem,
            rc,
        };

        ppmd.restart_model();

        Ok(ppmd)
    }

    #[inline(always)]
    fn is_real_context(&self, successor: Successor) -> bool {
        successor.is_real_context(self.units_start)
    }

    fn insert_node(&mut self, node: UnitRef, index: u32) {
        self.mem.set_stamp(node, EMPTY_NODE);
        self.mem.set_node_next(node, self.free_list[index as usize]);
        self.mem
            .set_nu(node, self.index2units[index as usize] as u32);
        self.free_list[index as usize] = node;
        self.stamps[index as usize] = self.stamps[index as usize].wrapping_add(1);
    }

    /// Removes the first node from a free list. The list must hold at least one node.
    fn remove_node(&mut self, index: u32) -> UnitRef {
        let index = index as usize;
        let node = self.free_list[index];
        self.free_list[index] = self.mem.node_next(node);
        self.stamps[index] = self.stamps[index].wrapping_sub(1);
        node
    }

    fn split_block(&mut self, ptr: UnitRef, old_index: u32, new_index: u32) {
        let nu = self.index2units[old_index as usize] as u32
            - self.index2units[new_index as usize] as u32;
        let ptr = ptr.add_units(self.index2units[new_index as usize] as u32);
        let mut index = self.units2index[(nu as usize) - 1] as u32;
        if self.index2units[index as usize] as u32 != nu {
            index -= 1;
            let k = self.index2units[index as usize] as u32;
            self.insert_node(ptr.add_units(k), nu.wrapping_sub(k).wrapping_sub(1));
        }
        self.insert_node(ptr, index);
    }

    /// We use the first u32 field of the 12-bytes as record type stamp.
    /// State   { symbol: u8, freq: u8, .. : freq != 0
    /// Context { num_stats: u8, ..        : flags or summ_freq != 0
    /// Node    { stamp: u32               : stamp == EMPTY_NODE for free record
    ///                                    : stamp == 0 for guard
    /// Last 12-bytes record in array is always containing the 12-bytes order-0 Context
    /// record.
    fn glue_free_blocks(&mut self) {
        self.glue_count = 1 << 13;
        self.stamps = [0; 38];

        // We set guard NODE at lo_unit.
        if self.lo_unit != self.hi_unit {
            self.mem.set_stamp(UnitRef::at(self.lo_unit), 0);
        }

        let head = self.glue_blocks();
        self.fill_list(head);
    }

    /// Glue free blocks. Returns the head of the list of glued blocks.
    fn glue_blocks(&mut self) -> UnitRef {
        let mut head = UnitRef::NULL;
        // The node whose next field links to the current node. `None` is the list head.
        let mut prev: Option<UnitRef> = None;
        for i in 0..PPMD_NUM_INDEXES as usize {
            let mut next = self.free_list[i];
            self.free_list[i] = UnitRef::NULL;
            while !next.is_null() {
                let node = next;
                let mut nu = self.mem.nu(node);
                match prev {
                    None => head = next,
                    Some(prev) => self.mem.set_node_next(prev, next),
                }
                next = self.mem.node_next(node);
                if nu != 0 {
                    prev = Some(node);
                    loop {
                        let node2 = node.add_units(nu);
                        if self.mem.stamp(node2) != EMPTY_NODE {
                            break;
                        }
                        nu += self.mem.nu(node2);
                        self.mem.set_nu(node, nu);
                        self.mem.set_nu(node2, 0);
                    }
                }
            }
        }
        match prev {
            None => head = UnitRef::NULL,
            Some(prev) => self.mem.set_node_next(prev, UnitRef::NULL),
        }
        head
    }

    /// Fill lists of free blocks.
    fn fill_list(&mut self, mut n: UnitRef) {
        while !n.is_null() {
            let mut node = n;
            let mut nu = self.mem.nu(node);
            n = self.mem.node_next(node);
            if nu == 0 {
                continue;
            }
            while nu > 128 {
                self.insert_node(node, PPMD_NUM_INDEXES - 1);
                nu -= 128;
                node = node.add_units(128);
            }
            let mut index = self.units2index[(nu as usize) - 1] as u32;
            if self.index2units[index as usize] as u32 != nu {
                index -= 1;
                let k = self.index2units[index as usize] as u32;
                self.insert_node(node.add_units(k), nu.wrapping_sub(k).wrapping_sub(1));
            }
            self.insert_node(node, index);
        }
    }

    #[inline(never)]
    fn alloc_units_rare(&mut self, index: u32) -> Option<UnitRef> {
        if self.glue_count == 0 {
            self.glue_free_blocks();
            if !self.free_list[index as usize].is_null() {
                return Some(self.remove_node(index));
            }
        }

        let mut i = index;

        loop {
            i += 1;
            if i == PPMD_NUM_INDEXES {
                let num_bytes = self.index2units[index as usize] as u32 * UNIT_SIZE;
                let us = self.units_start;
                self.glue_count -= 1;
                return if us - self.text > num_bytes {
                    self.units_start = us - num_bytes;
                    Some(UnitRef::at(self.units_start))
                } else {
                    None
                };
            }
            if !self.free_list[i as usize].is_null() {
                break;
            }
        }

        let block = self.remove_node(i);
        self.split_block(block, i, index);
        Some(block)
    }

    fn alloc_units(&mut self, index: u32) -> Option<UnitRef> {
        if !self.free_list[index as usize].is_null() {
            return Some(self.remove_node(index));
        }
        let num_bytes = self.index2units[index as usize] as u32 * UNIT_SIZE;
        let lo = self.lo_unit;
        if self.hi_unit - lo >= num_bytes {
            self.lo_unit = lo + num_bytes;
            return Some(UnitRef::at(lo));
        }
        self.alloc_units_rare(index)
    }

    fn shrink_units(&mut self, old_ptr: UnitRef, old_nu: u32, new_nu: u32) -> UnitRef {
        let i0 = self.units2index[(old_nu as usize) - 1] as u32;
        let i1 = self.units2index[(new_nu as usize) - 1] as u32;

        if i0 == i1 {
            return old_ptr;
        }

        if !self.free_list[i1 as usize].is_null() {
            let ptr = self.remove_node(i1);
            self.mem.copy_units(old_ptr, ptr, new_nu);
            self.insert_node(old_ptr, i0);
            return ptr;
        }

        self.split_block(old_ptr, i0, i1);

        old_ptr
    }

    fn free_units(&mut self, ptr: UnitRef, nu: u32) {
        self.insert_node(ptr, self.units2index[(nu as usize) - 1] as u32);
    }

    fn special_free_unit(&mut self, ptr: UnitRef) {
        if ptr.offset() != self.units_start {
            self.insert_node(ptr, 0);
        } else {
            self.units_start += UNIT_SIZE;
        }
    }

    fn expand_text_area(&mut self) {
        let mut count: [u32; 38] = [0; 38];

        if self.lo_unit != self.hi_unit {
            self.mem.set_stamp(UnitRef::at(self.lo_unit), 0);
        }

        let mut node = UnitRef::at(self.units_start);
        while self.mem.stamp(node) == EMPTY_NODE {
            let nu = self.mem.nu(node);
            self.mem.set_stamp(node, 0);
            count[self.units2index[(nu as usize) - 1] as usize] += 1;
            node = node.add_units(nu);
        }
        self.units_start = node.offset();

        for (i, mut cnt) in count.into_iter().enumerate() {
            if cnt != 0 {
                // The node whose next field links to the current node. `None` is the list head.
                let mut prev: Option<UnitRef> = None;
                let mut n = self.free_list[i];
                self.stamps[i] = self.stamps[i].wrapping_sub(cnt);
                loop {
                    let node = n;
                    n = self.mem.node_next(node);
                    if self.mem.stamp(node) != 0 {
                        prev = Some(node);
                        continue;
                    }

                    match prev {
                        None => self.free_list[i] = n,
                        Some(prev) => self.mem.set_node_next(prev, n),
                    }
                    cnt -= 1;
                    if cnt == 0 {
                        break;
                    }
                }
            }
        }
    }

    #[inline(never)]
    fn restart_model(&mut self) {
        self.free_list = [UnitRef::NULL; 38];
        self.stamps = [0; 38];

        // The end of the memory is 2^32 for the largest memory size. The wrapping additions
        // produce the correct offsets as soon as the unit at the end is subtracted.
        self.text = self.align_offset;
        self.hi_unit = self.text.wrapping_add(self.size);
        self.units_start = self
            .hi_unit
            .wrapping_sub(self.size / 8 / UNIT_SIZE * 7 * UNIT_SIZE);
        self.lo_unit = self.units_start;
        self.glue_count = 0;

        self.order_fall = self.max_order;
        self.init_rl = -(if self.max_order < 12 {
            self.max_order as i32
        } else {
            12
        }) - 1;
        self.run_length = self.init_rl;
        self.prev_success = 0;

        self.hi_unit = self.hi_unit.wrapping_sub(UNIT_SIZE);
        let mc = UnitRef::at(self.hi_unit).as_ctx();
        let s = UnitRef::at(self.lo_unit).as_states();

        self.lo_unit += (256 / 2) * UNIT_SIZE;
        self.min_context = mc;
        self.max_context = mc;
        self.found_state = s;

        self.mem.set_flags(mc, 0);
        self.mem.set_num_stats(mc, 256 - 1);
        self.mem.set_summ_freq(mc, 256 + 1);
        self.mem.set_stats(mc, s);
        self.mem.set_suffix(mc, CtxRef::NULL);

        (0..256).for_each(|i| {
            let s = s.add(i);
            self.mem.set_symbol(s, i as u8);
            self.mem.set_freq(s, 1);
            self.mem.set_successor(s, Successor::NULL);
        });

        let mut i = 0;
        (0..25).for_each(|m| {
            while self.ns2index[i as usize] as usize == m {
                i += 1;
            }

            (0..8).for_each(|k| {
                let val = PPMD_BIN_SCALE - (K_INIT_BIN_ESC[k] as u32) / (i + 1);

                (0..64).step_by(8).for_each(|r| {
                    self.bin_summ[m][k + r] = val as u16;
                });
            });
        });

        let mut i = 0;
        (0..24).for_each(|m| {
            while self.ns2index[(i + 3) as usize] as usize == m + 3 {
                i += 1;
            }

            let summ = (2 * i + 5) << (PPMD_PERIOD_BITS - 4);

            (0..32).for_each(|k| {
                let see = &mut self.see[m][k];
                see.summ = summ as u16;
                see.shift = (PPMD_PERIOD_BITS - 4) as u8;
                see.count = 7;
            });
        });

        self.dummy_see.summ = 0; // unused
        self.dummy_see.shift = PPMD_PERIOD_BITS as u8;
        self.dummy_see.count = 64; // unused
    }

    /// Returns the state with the given symbol in the stats of a context with more than one
    /// state.
    #[inline(always)]
    fn find_state(&self, ctx: CtxRef, symbol: u8) -> StateRef {
        let mut s = self.mem.stats(ctx);
        while self.mem.symbol(s) != symbol {
            s = s.next();
        }
        s
    }

    /// Converts a context into a binary context with the state `s` as its one state.
    fn set_one_state_from(&mut self, ctx: CtxRef, s: StateRef) {
        let symbol = self.mem.symbol(s);
        self.mem.set_flags(
            ctx,
            ((self.mem.flags(ctx) & FLAG_PREV_HIGH) as u32 + Self::hi_bits_flag3(symbol as u32))
                as u8,
        );
        let one_state = ctx.one_state();
        self.mem.set_symbol(one_state, symbol);
        self.mem
            .set_freq(one_state, ((self.mem.freq(s) as u32 + 11) >> 3) as u8);
        self.mem.set_successor(one_state, self.mem.successor(s));
    }

    /// This function is called when we remove some symbols (successors) in context.
    /// It increases escape_freq for sum of all removed symbols.
    fn refresh(&mut self, ctx: CtxRef, old_nu: u32, mut scale: u32) {
        let num_stats = self.mem.num_stats(ctx);

        let states = self.mem.stats(ctx).as_unit();
        let mut s = self
            .shrink_units(states, old_nu, (num_stats + 2) >> 1)
            .as_states();
        self.mem.set_stats(ctx, s);

        scale |= (self.mem.summ_freq(ctx) >= 1 << 15) as u32;

        let mut flags = Self::hi_bits_prepare(self.mem.symbol(s) as u32);
        let mut freq = self.mem.freq(s) as u32;
        let mut esc_freq = self.mem.summ_freq(ctx) - freq;
        freq = freq.wrapping_add(scale) >> scale;
        let mut sum_freq = freq;
        self.mem.set_freq(s, freq as u8);

        let mut i = num_stats;

        loop {
            s = s.next();
            let mut freq = self.mem.freq(s) as u32;
            esc_freq -= freq;
            freq = freq.wrapping_add(scale) >> scale;
            sum_freq += freq;
            self.mem.set_freq(s, freq as u8);
            flags |= Self::hi_bits_prepare(self.mem.symbol(s) as u32);

            i -= 1;
            if i == 0 {
                break;
            }
        }

        self.mem
            .set_summ_freq(ctx, sum_freq + ((esc_freq + scale) >> scale));
        self.mem.set_flags(
            ctx,
            ((self.mem.flags(ctx) as u32 & (FLAG_PREV_HIGH as u32 + FLAG_RESCALED as u32 * scale))
                + Self::hi_bits_convert_3(flags)) as u8,
        );
    }

    /// Reduces contexts.
    ///
    /// It converts successors at max_order to another contexts to NULL-successors.
    /// It removes RAW-successors and NULL-successors that are not Order-0, and it
    /// removes contexts when it has no successors. If the (multi_state.stats) is
    /// close to (units_start), it moves it up.
    fn cut_off(&mut self, ctx: CtxRef, order: u32) -> CtxRef {
        let mut ns = self.mem.num_stats(ctx) as i32;

        if ns == 0 {
            let one_state = ctx.one_state();
            let successor = self.mem.successor(one_state);
            if self.is_real_context(successor) {
                let successor = if order < self.max_order {
                    self.cut_off(successor.context(), order + 1)
                } else {
                    CtxRef::NULL
                };
                self.mem
                    .set_successor(one_state, Successor::from(successor));
                if !successor.is_null() || order <= 9 {
                    // O_BOUND
                    return ctx;
                }
            }
            self.special_free_unit(ctx.as_unit());
            return CtxRef::NULL;
        }

        let nu = (ns as u32).wrapping_add(2) >> 1;

        let index = self.units2index[(nu as usize) - 1] as u32;
        let mut stats = self.mem.stats(ctx);

        if stats.offset().wrapping_sub(self.units_start) <= (1 << 14)
            && stats.offset() <= self.free_list[index as usize].offset()
        {
            let ptr = self.remove_node(index);
            self.mem.set_stats(ctx, ptr.as_states());
            self.mem.copy_units(stats.as_unit(), ptr, nu);

            if stats.offset() != self.units_start {
                self.insert_node(stats.as_unit(), index);
            } else {
                self.units_start += self.index2units[index as usize] as u32 * UNIT_SIZE;
            }
            stats = ptr.as_states();
        }

        let mut s = stats.add(ns as u32);

        loop {
            let successor = self.mem.successor(s);
            if !self.is_real_context(successor) {
                let s2 = stats.add(ns as u32);
                ns -= 1;
                if order != 0 {
                    if s != s2 {
                        self.mem.copy_state(s2, s);
                    }
                } else {
                    self.mem.swap_states(s, s2);
                    self.mem.set_successor(s2, Successor::NULL);
                }
            } else if order < self.max_order {
                let successor = self.cut_off(successor.context(), order + 1);
                self.mem.set_successor(s, Successor::from(successor));
            } else {
                self.mem.set_successor(s, Successor::NULL);
            }

            if s == stats {
                break;
            }
            s = s.prev();
        }

        if ns != self.mem.num_stats(ctx) as i32 && order != 0 {
            if ns < 0 {
                self.free_units(stats.as_unit(), nu);
                self.special_free_unit(ctx.as_unit());
                return CtxRef::NULL;
            }
            self.mem.set_num_stats(ctx, ns as u32);
            if ns == 0 {
                self.set_one_state_from(ctx, stats);
                self.free_units(stats.as_unit(), nu);
            } else {
                let scale = (self.mem.summ_freq(ctx) > 16 * (ns as u32)) as u32;
                self.refresh(ctx, nu, scale);
            }
        }

        ctx
    }

    fn get_used_memory(&self) -> u32 {
        let mut v = 0;

        for i in 0..PPMD_NUM_INDEXES {
            v += self.stamps[i as usize] * self.index2units[i as usize] as u32;
        }

        self.size - (self.hi_unit - self.lo_unit) - (self.units_start - self.text) - (v * UNIT_SIZE)
    }

    fn restore_model(&mut self, ctx_error: CtxRef) {
        self.text = self.align_offset;

        // We go here in cases of error of allocation for context (c1)
        // Order(min_context) < Order(ctx_error) <= Order(max_context)

        let mut c = self.max_context;

        // We remove last symbol from each of contexts [p.max_context ... ctx_error) contexts.
        // So we roll back all created (symbols) before the error.
        while c != ctx_error {
            let num_stats = self.mem.num_stats(c) - 1;
            self.mem.set_num_stats(c, num_stats);
            if num_stats == 0 {
                let s = self.mem.stats(c);
                self.set_one_state_from(c, s);
                self.special_free_unit(s.as_unit());
            } else {
                // refresh() can increase escape_freq on value of freq of last symbol,
                // that was added before error. So the largest possible increase for escape_freq
                // is (8) from value before model_update()
                self.refresh(c, (num_stats + 3) >> 1, 0);
            }

            c = self.mem.suffix(c);
        }

        // Increase escape_freq for context [ctx_error..p.MinContext)
        while c != self.min_context {
            let num_stats = self.mem.num_stats(c);
            if num_stats == 0 {
                let one_state = c.one_state();
                self.mem.set_freq(
                    one_state,
                    ((self.mem.freq(one_state) as u32 + 1) >> 1) as u8,
                );
            } else {
                self.mem.set_summ_freq(c, self.mem.summ_freq(c) + 4);
                if self.mem.summ_freq(c) > 128 + 4 * num_stats {
                    self.refresh(c, num_stats.wrapping_add(2) >> 1, 1);
                }
            }

            c = self.mem.suffix(c);
        }

        if self.restore_method == RestoreMethod::Restart || self.get_used_memory() < self.size >> 1
        {
            self.restart_model();
        } else {
            while !self.mem.suffix(self.max_context).is_null() {
                self.max_context = self.mem.suffix(self.max_context);
            }
            loop {
                self.cut_off(self.max_context, 0);
                self.expand_text_area();

                if self.get_used_memory() <= 3 * (self.size >> 2) {
                    break;
                }
            }
            self.glue_count = 0;
            self.order_fall = self.max_order;
        }
        self.min_context = self.max_context;
    }

    #[inline(never)]
    fn create_successors(
        &mut self,
        skip: bool,
        s1: &mut Option<StateRef>,
        mut c: CtxRef,
    ) -> Option<CtxRef> {
        let up_branch = self.mem.successor(self.found_state);
        let mut num_ps = 0;
        // Fixed over Shkarin's code. Maybe it could work without + 1 too.
        let mut ps = [StateRef::NULL; PPMD8_MAX_ORDER as usize + 1];

        if !skip {
            ps[num_ps] = self.found_state;
            num_ps += 1;
        }

        while !self.mem.suffix(c).is_null() {
            c = self.mem.suffix(c);

            let s = if let Some(state) = s1.take() {
                state
            } else if self.mem.num_stats(c) != 0 {
                let s = self.find_state(c, self.mem.symbol(self.found_state));
                let freq = self.mem.freq(s);
                if freq < MAX_FREQ - 9 {
                    self.mem.set_freq(s, freq + 1);
                    self.mem.set_summ_freq(c, self.mem.summ_freq(c) + 1);
                }
                s
            } else {
                let s = c.one_state();
                let freq = self.mem.freq(s) as u32;
                let suffix_num_stats = self.mem.num_stats(self.mem.suffix(c));
                self.mem.set_freq(
                    s,
                    (freq + ((suffix_num_stats == 0) as u32 & (freq < 24) as u32)) as u8,
                );
                s
            };

            let successor = self.mem.successor(s);
            if successor != up_branch {
                c = successor.context();
                if num_ps == 0 {
                    return Some(c);
                }
                break;
            }
            ps[num_ps] = s;
            num_ps += 1;
        }

        let new_sym = self.mem.text(up_branch.text_position());
        let up_branch = Successor::text(up_branch.text_position() + 1);

        let flags = (Self::hi_bits_flag4(self.mem.symbol(self.found_state) as u32)
            + Self::hi_bits_flag3(new_sym as u32)) as u8;

        let new_freq = if self.mem.num_stats(c) == 0 {
            self.mem.freq(c.one_state())
        } else {
            let s = self.find_state(c, new_sym);
            let cf = (self.mem.freq(s) as u32) - 1;
            let s0 = self.mem.summ_freq(c) - self.mem.num_stats(c) - cf;
            1 + (if 2 * cf <= s0 {
                (5 * cf > s0) as u32
            } else {
                (cf + (2 * s0) - 3) / s0
            }) as u8
        };

        loop {
            let c1 = if self.hi_unit != self.lo_unit {
                self.hi_unit -= UNIT_SIZE;
                UnitRef::at(self.hi_unit)
            } else if !self.free_list[0].is_null() {
                self.remove_node(0)
            } else {
                self.alloc_units_rare(0)?
            }
            .as_ctx();

            self.mem.set_flags(c1, flags);
            self.mem.set_num_stats(c1, 0);
            let one_state = c1.one_state();
            self.mem.set_symbol(one_state, new_sym);
            self.mem.set_freq(one_state, new_freq);
            self.mem.set_successor(one_state, up_branch);
            self.mem.set_suffix(c1, c);

            num_ps -= 1;
            self.mem.set_successor(ps[num_ps], Successor::from(c1));

            c = c1;
            if num_ps == 0 {
                break;
            }
        }

        Some(c)
    }

    fn reduce_order(&mut self, mut s1: Option<StateRef>, mut c: CtxRef) -> Option<CtxRef> {
        let mut s;
        let c1 = c;
        let up_branch = Successor::text(self.text);

        self.mem.set_successor(self.found_state, up_branch);
        self.order_fall += 1;

        loop {
            if let Some(state) = s1.take() {
                c = self.mem.suffix(c);
                s = state;
            } else {
                if self.mem.suffix(c).is_null() {
                    return Some(c);
                }
                c = self.mem.suffix(c);
                if self.mem.num_stats(c) != 0 {
                    s = self.find_state(c, self.mem.symbol(self.found_state));
                    let freq = self.mem.freq(s);
                    if freq < MAX_FREQ - 9 {
                        self.mem.set_freq(s, freq + 2);
                        self.mem.set_summ_freq(c, self.mem.summ_freq(c) + 2);
                    }
                } else {
                    s = c.one_state();
                    let freq = self.mem.freq(s);
                    self.mem.set_freq(s, freq + (freq < 32) as u8);
                }
            }
            if !self.mem.successor(s).is_null() {
                break;
            }
            self.mem.set_successor(s, up_branch);
            self.order_fall += 1;
        }

        if self.mem.successor(s) <= up_branch {
            let s2 = self.found_state;
            self.found_state = s;
            let successor = self
                .create_successors(false, &mut None, c)
                .unwrap_or(CtxRef::NULL);
            self.mem.set_successor(s, Successor::from(successor));
            self.found_state = s2;
        }

        let successor = self.mem.successor(s);
        if self.order_fall == 1 && c1 == self.max_context {
            self.mem.set_successor(self.found_state, successor);
            self.text -= 1;
        }
        if successor.is_null() {
            return None;
        }

        Some(successor.context())
    }

    #[inline(never)]
    fn update_model(&mut self) {
        let mut min_successor = self.mem.successor(self.found_state);
        let f_freq = self.mem.freq(self.found_state) as u32;
        let f_symbol = self.mem.symbol(self.found_state);
        let mut s: Option<StateRef> = None;

        if f_freq < (MAX_FREQ / 4) as u32 && !self.mem.suffix(self.min_context).is_null() {
            // Update frequencies in suffix Context
            let c = self.mem.suffix(self.min_context);
            if self.mem.num_stats(c) == 0 {
                let state = c.one_state();
                let freq = self.mem.freq(state);
                if freq < 32 {
                    self.mem.set_freq(state, freq + 1);
                }
                s = Some(state);
            } else {
                let mut state = self.mem.stats(c);

                if self.mem.symbol(state) != f_symbol {
                    state = self.find_state(c, f_symbol);
                    if self.mem.freq(state) >= self.mem.freq(state.prev()) {
                        self.mem.swap_states(state, state.prev());
                        state = state.prev();
                    }
                }

                let freq = self.mem.freq(state);
                if freq < MAX_FREQ - 9 {
                    self.mem.set_freq(state, freq + 2);
                    self.mem.set_summ_freq(c, self.mem.summ_freq(c) + 2);
                }

                s = Some(state);
            }
        }

        let mut c = self.max_context;
        if self.order_fall == 0 && !min_successor.is_null() {
            let Some(cs) = self.create_successors(true, &mut s, self.min_context) else {
                self.mem.set_successor(self.found_state, Successor::NULL);
                self.restore_model(c);
                return;
            };

            self.mem
                .set_successor(self.found_state, Successor::from(cs));
            self.max_context = cs;
            self.min_context = cs;
            return;
        }

        self.mem.set_text(self.text, f_symbol);
        self.text += 1;
        if self.text >= self.units_start {
            self.restore_model(c);
            return;
        }
        let mut max_successor = Successor::text(self.text);

        if min_successor.is_null() {
            let Some(cs) = self.reduce_order(s, self.min_context) else {
                self.restore_model(c);
                return;
            };
            min_successor = Successor::from(cs);
        } else if !self.is_real_context(min_successor) {
            let Some(cs) = self.create_successors(false, &mut s, self.min_context) else {
                self.restore_model(c);
                return;
            };
            min_successor = Successor::from(cs);
        }

        self.order_fall -= 1;
        if self.order_fall == 0 {
            max_successor = min_successor;
            self.text -= (self.max_context != self.min_context) as u32;
        }

        let flag = Self::hi_bits_flag3(f_symbol as u32) as u8;
        let ns = self.mem.num_stats(self.min_context);
        let s0 = self.mem.summ_freq(self.min_context) - ns - f_freq;

        while c != self.min_context {
            let mut sum;
            let ns1 = self.mem.num_stats(c);
            if ns1 != 0 {
                if ns1 & 1 != 0 {
                    // Expand for one UNIT
                    let old_nu = (ns1 + 1) >> 1;
                    let i = self.units2index[(old_nu as usize) - 1] as u32;
                    if i != self.units2index[old_nu as usize] as u32 {
                        let Some(ptr) = self.alloc_units(i.wrapping_add(1)) else {
                            self.restore_model(c);
                            return;
                        };
                        let old_ptr = self.mem.stats(c).as_unit();
                        self.mem.copy_units(old_ptr, ptr, old_nu);
                        self.insert_node(old_ptr, i);
                        self.mem.set_stats(c, ptr.as_states());
                    }
                }
                sum = self.mem.summ_freq(c);
                // max increase of escape_freq is 1 here.
                // An average increase is 1/3 per symbol
                sum = sum.wrapping_add(((3 * ns1 + 1) < ns) as u32);
            } else {
                let Some(s) = self.alloc_units(0) else {
                    self.restore_model(c);
                    return;
                };
                let s = s.as_states();

                let one_state = c.one_state();
                let mut freq = self.mem.freq(one_state) as u32;
                self.mem.set_symbol(s, self.mem.symbol(one_state));
                self.mem.set_successor(s, self.mem.successor(one_state));
                self.mem.set_stats(c, s);

                if freq < (MAX_FREQ as u32 / 4 - 1) {
                    freq <<= 1;
                } else {
                    freq = MAX_FREQ as u32 - 4;
                }
                self.mem.set_freq(s, freq as u8);

                sum = freq + self.init_esc + (ns > 2) as u32;
            }

            let s = self.mem.stats(c).add(ns1 + 1);
            let mut cf = 2 * sum.wrapping_add(6) * f_freq;
            let sf = s0.wrapping_add(sum);
            self.mem.set_symbol(s, f_symbol);
            self.mem.set_num_stats(c, ns1.wrapping_add(1));
            self.mem.set_successor(s, max_successor);
            self.mem.set_flags(c, self.mem.flags(c) | flag);
            if cf < 6 * sf {
                cf = 1 + (cf > sf) as u32 + (cf >= 4 * sf) as u32;
                sum = sum.wrapping_add(4);
                // It can add (1, 2, 3) to escape_freq
            } else {
                cf = 4u32 + (cf > 9 * sf) as u32 + (cf > 12 * sf) as u32 + (cf > 15 * sf) as u32;
                sum = sum.wrapping_add(cf);
            }

            self.mem.set_summ_freq(c, sum);
            self.mem.set_freq(s, cf as u8);
            c = self.mem.suffix(c);
        }

        self.min_context = min_successor.context();
        self.max_context = self.min_context;
    }

    #[inline(never)]
    fn rescale(&mut self) {
        let mc = self.min_context;
        let stats = self.mem.stats(mc);
        let mut s = self.found_state;

        // Sort the list by freq
        if s != stats {
            let tmp = self.mem.state(s);
            loop {
                self.mem.copy_state(s.prev(), s);
                s = s.prev();
                if s == stats {
                    break;
                }
            }
            self.mem.set_state(s, tmp);
        }

        let mut sum_freq = self.mem.freq(s) as u32;
        let mut esc_freq = self.mem.summ_freq(mc) - sum_freq;

        let adder = (self.order_fall != 0) as u32;

        sum_freq = (sum_freq + 4 + adder) >> 1;
        self.mem.set_freq(s, sum_freq as u8);

        let mut i = self.mem.num_stats(mc);

        loop {
            s = s.next();
            let mut freq = self.mem.freq(s) as u32;
            esc_freq -= freq;
            freq = freq.wrapping_add(adder) >> 1;
            sum_freq += freq;
            self.mem.set_freq(s, freq as u8);

            if freq > self.mem.freq(s.prev()) as u32 {
                let tmp = self.mem.state(s);
                let mut s1 = s;
                loop {
                    self.mem.copy_state(s1.prev(), s1);
                    s1 = s1.prev();
                    if !(s1 != stats && freq > self.mem.freq(s1.prev()) as u32) {
                        break;
                    }
                }
                self.mem.set_state(s1, tmp);
            }
            i -= 1;
            if i == 0 {
                break;
            }
        }

        if self.mem.freq(s) == 0 {
            // Remove all items with freq == 0
            let mut i = 0;
            loop {
                i += 1;
                s = s.prev();
                if self.mem.freq(s) != 0 {
                    break;
                }
            }

            esc_freq += i;
            let num_stats = self.mem.num_stats(mc);
            let num_stats_new = num_stats - i;
            self.mem.set_num_stats(mc, num_stats_new);
            let n0 = (num_stats + 2) >> 1;

            if num_stats_new == 0 {
                let mut freq = (2u32 * (self.mem.freq(stats) as u32)).div_ceil(esc_freq);
                if freq > (MAX_FREQ / 3) as u32 {
                    freq = (MAX_FREQ / 3) as u32;
                }
                self.mem.set_flags(
                    mc,
                    ((self.mem.flags(mc) & FLAG_PREV_HIGH) as u32
                        + Self::hi_bits_flag3(self.mem.symbol(stats) as u32))
                        as u8,
                );

                let s = mc.one_state();
                self.mem.copy_state(stats, s);
                self.mem.set_freq(s, freq as u8);
                self.found_state = s;
                self.insert_node(stats.as_unit(), self.units2index[(n0 as usize) - 1] as u32);
                return;
            }

            let n1 = (num_stats_new + 2) >> 1;
            if n0 != n1 {
                let shrunk = self.shrink_units(stats.as_unit(), n0, n1);
                self.mem.set_stats(mc, shrunk.as_states());
            }
        }

        self.mem.set_summ_freq(
            mc,
            sum_freq.wrapping_add(esc_freq).wrapping_sub(esc_freq >> 1),
        );
        self.mem.set_flags(mc, self.mem.flags(mc) | FLAG_RESCALED);
        self.found_state = self.mem.stats(mc);
    }

    fn make_esc_freq(&mut self, num_masked: u32, esc_freq: &mut u32) -> SeeSource {
        let num_stats = self.mem.num_stats(self.min_context);

        if num_stats != 0xFF {
            let (base_context_idx, see_table_hash) =
                self.calculate_see_table_hash(num_masked, num_stats);

            let see = &mut self.see[base_context_idx][see_table_hash];

            // If (see.summ) field is larger than 16-bit, we need only low 16 bits of summ.
            let summ = see.summ as u32;
            let r = summ >> see.shift as i32;
            see.summ = (summ - r) as u16;
            *esc_freq = r + (r == 0) as u32;

            SeeSource::Table(base_context_idx, see_table_hash)
        } else {
            *esc_freq = 1;
            SeeSource::Dummy
        }
    }

    fn calculate_see_table_hash(&self, num_masked: u32, num_stats: u32) -> (usize, usize) {
        let mc = self.min_context;
        let base_context_idx = self.ns2index[(num_stats + 2) as usize] as usize - 3;

        let suffix_num_stats = self.mem.num_stats(self.mem.suffix(mc));
        let summ_freq = self.mem.summ_freq(mc);

        let freq_distribution_hash = (summ_freq > 11 * (num_stats + 1)) as usize;
        let context_hierarchy_hash =
            2 * ((2 * num_stats) < (suffix_num_stats + num_masked)) as usize;
        let symbol_characteristics_hash = self.mem.flags(mc) as usize;

        let see_table_hash =
            freq_distribution_hash + context_hierarchy_hash + symbol_characteristics_hash;

        (base_context_idx, see_table_hash)
    }

    fn get_see(&mut self, see_source: SeeSource) -> &mut See {
        match see_source {
            SeeSource::Dummy => &mut self.dummy_see,
            SeeSource::Table(i, k) => &mut self.see[i][k],
        }
    }

    fn next_context(&mut self) {
        let successor = self.mem.successor(self.found_state);
        if self.order_fall == 0 && self.is_real_context(successor) {
            self.min_context = successor.context();
            self.max_context = successor.context();
        } else {
            self.update_model();
        }
    }

    fn update1(&mut self) {
        let mut s = self.found_state;
        let mc = self.min_context;
        let freq = self.mem.freq(s) as u32 + 4;
        self.mem.set_summ_freq(mc, self.mem.summ_freq(mc) + 4);
        self.mem.set_freq(s, freq as u8);
        if freq > self.mem.freq(s.prev()) as u32 {
            self.mem.swap_states(s, s.prev());
            s = s.prev();
            self.found_state = s;
            if freq > MAX_FREQ as u32 {
                self.rescale();
            }
        }
        self.next_context();
    }

    fn update1_0(&mut self) {
        let s = self.found_state;
        let mc = self.min_context;
        let mut freq = self.mem.freq(s) as u32;
        let summ_freq = self.mem.summ_freq(mc);
        self.prev_success = (2 * freq >= summ_freq) as u32; // Ppmd8 (>=)
        self.run_length += self.prev_success as i32;
        self.mem.set_summ_freq(mc, summ_freq + 4);
        freq += 4;
        self.mem.set_freq(s, freq as u8);
        if freq > MAX_FREQ as u32 {
            self.rescale();
        }
        self.next_context();
    }

    fn update2(&mut self) {
        let s = self.found_state;
        let mc = self.min_context;
        let freq = self.mem.freq(s) as u32 + 4;
        self.run_length = self.init_rl;
        self.mem.set_summ_freq(mc, self.mem.summ_freq(mc) + 4);
        self.mem.set_freq(s, freq as u8);
        if freq > MAX_FREQ as u32 {
            self.rescale();
        }
        self.update_model();
    }

    fn update_bin(&mut self, s: StateRef) -> u8 {
        let freq = self.mem.freq(s);
        let sym = self.mem.symbol(s);
        self.found_state = s;
        self.prev_success = 1;
        self.run_length += 1;
        self.mem.set_freq(s, freq.wrapping_add((freq < 196) as u8));
        self.next_context();
        sym
    }

    fn mask_symbols(&self, char_mask: &mut [u8; 256], s: StateRef, mut s2: StateRef) {
        char_mask[self.mem.symbol(s) as usize] = 0;
        loop {
            let sym0 = self.mem.symbol(s2);
            let sym1 = self.mem.symbol(s2.next());
            s2 = s2.add(2);
            char_mask[sym0 as usize] = 0;
            char_mask[sym1 as usize] = 0;

            if s2 >= s {
                break;
            }
        }
    }

    const fn hi_bits_prepare(flag: u32) -> u32 {
        flag + 0xC0
    }

    const fn hi_bits_convert_3(flag: u32) -> u32 {
        flag >> (8 - 3) & (1 << 3)
    }

    const fn hi_bits_convert_4(flag: u32) -> u32 {
        flag >> (8 - 4) & (1 << 4)
    }

    const fn hi_bits_flag3(symbol: u32) -> u32 {
        Self::hi_bits_convert_3(Self::hi_bits_prepare(symbol))
    }

    const fn hi_bits_flag4(symbol: u32) -> u32 {
        Self::hi_bits_convert_4(Self::hi_bits_prepare(symbol))
    }

    fn get_bin_summ(&mut self) -> &mut u16 {
        let mc = self.min_context;
        let freq = self.mem.freq(mc.one_state()) as usize;
        let freq_bin_idx = self.ns2index[freq - 1] as usize;

        let num_stats = self.mem.num_stats(self.mem.suffix(mc)) as usize;

        let context_idx = self
            .prev_success
            .wrapping_add(self.run_length as u32 >> 26 & 0x20)
            .wrapping_add(self.ns2bs_index[num_stats] as u32)
            .wrapping_add(self.mem.flags(mc) as u32) as usize;

        &mut self.bin_summ[freq_bin_idx][context_idx]
    }
}

impl<R: Read> PPMd8<RangeDecoder<R>> {
    pub(crate) fn new_decoder(
        reader: R,
        max_order: u32,
        mem_size: u32,
        restore_method: RestoreMethod,
    ) -> Result<Self, Error> {
        let range_decoder = RangeDecoder::new(reader)?;
        Self::construct(range_decoder, max_order, mem_size, restore_method)
    }

    /// Gets a reference to the underlying reader.
    pub(crate) fn get_ref(&self) -> &R {
        &self.rc.reader
    }

    /// Gets a mutable reference to the underlying reader.
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this decoder is continued to be used.
    pub(crate) fn get_mut(&mut self) -> &mut R {
        &mut self.rc.reader
    }

    pub(crate) fn into_inner(self) -> R {
        self.rc.reader
    }

    pub(crate) fn range_decoder_code(&self) -> u32 {
        self.rc.code
    }
}

impl<W: Write> PPMd8<RangeEncoder<W>> {
    pub(crate) fn new_encoder(
        writer: W,
        max_order: u32,
        mem_size: u32,
        restore_method: RestoreMethod,
    ) -> Result<Self, Error> {
        let range_encoder = RangeEncoder::new(writer);
        Self::construct(range_encoder, max_order, mem_size, restore_method)
    }

    /// Gets a reference to the underlying writer.
    pub(crate) fn get_ref(&self) -> &W {
        &self.rc.writer
    }

    /// Gets a mutable reference to the underlying writer.
    ///
    /// Note that mutating the output/input state of the stream may corrupt
    /// this object, so care must be taken when using this method.
    pub(crate) fn get_mut(&mut self) -> &mut W {
        &mut self.rc.writer
    }

    pub(crate) fn into_inner(self) -> W {
        self.rc.writer
    }

    pub(crate) fn flush_range_encoder(&mut self) -> Result<(), std::io::Error> {
        self.rc.flush()
    }
}

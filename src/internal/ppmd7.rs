mod decoder;
mod encoder;
mod range_coding;

use std::io::{Read, Write};

pub(crate) use range_coding::{
    Ppmd7RangeDecoder, Ppmd7RangeEncoder, RangeDecoder, RangeDecoder7a, RangeEncoder,
    RangeEncoder7a,
};

use super::*;
use crate::Error;

const MAX_FREQ: u8 = 124;
const K_TOP_VALUE: u32 = 1 << 24;
const K_BOT_VALUE: u32 = 1 << 15;
const EMPTY_NODE: u16 = 0;

static K_EXP_ESCAPE: [u8; 16] = [25, 14, 9, 7, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 2, 2];

static K_INIT_BIN_ESC: [u16; 8] = [
    0x3CDD, 0x1F3F, 0x59BF, 0x48F3, 0x64A1, 0x5ABC, 0x6632, 0x6051,
];

/// The context and free node records of PPMd7.
///
/// A context record has 12 bytes:
///
/// | Offset | Field                                                      |
/// |--------|------------------------------------------------------------|
/// | 0      | `num_stats: u16`                                           |
/// | 2      | `summ_freq: u16`, or the one state of a binary context     |
/// | 4      | `stats: u32`, or the successor of the one state            |
/// | 8      | `suffix: u32`                                              |
///
/// A free node record has 12 bytes: `stamp: u16`, `nu: u16`, `next: u32`. The free list link
/// is a `u32` at offset 0 and overlaps `stamp` and `nu`.
trait Records {
    fn num_stats(&self, ctx: CtxRef) -> u32;
    fn set_num_stats(&mut self, ctx: CtxRef, value: u32);
    fn summ_freq(&self, ctx: CtxRef) -> u32;
    fn set_summ_freq(&mut self, ctx: CtxRef, value: u32);
    fn stats(&self, ctx: CtxRef) -> StateRef;
    fn set_stats(&mut self, ctx: CtxRef, value: StateRef);
    fn suffix(&self, ctx: CtxRef) -> CtxRef;
    fn set_suffix(&mut self, ctx: CtxRef, value: CtxRef);
    fn stamp(&self, node: UnitRef) -> u16;
    fn set_stamp(&mut self, node: UnitRef, value: u16);
    fn nu(&self, node: UnitRef) -> u32;
    fn set_nu(&mut self, node: UnitRef, value: u32);
    fn node_next(&self, node: UnitRef) -> UnitRef;
    fn set_node_next(&mut self, node: UnitRef, value: UnitRef);
    fn free_link(&self, node: UnitRef) -> UnitRef;
    fn set_free_link(&mut self, node: UnitRef, value: UnitRef);
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> Records for Memory<B> {
    #[inline(always)]
    fn num_stats(&self, ctx: CtxRef) -> u32 {
        self.unit_u16(ctx.offset(), 0) as u32
    }

    #[inline(always)]
    fn set_num_stats(&mut self, ctx: CtxRef, value: u32) {
        self.set_unit_u16(ctx.offset(), 0, value as u16);
    }

    #[inline(always)]
    fn summ_freq(&self, ctx: CtxRef) -> u32 {
        self.unit_u16(ctx.offset(), 2) as u32
    }

    #[inline(always)]
    fn set_summ_freq(&mut self, ctx: CtxRef, value: u32) {
        self.set_unit_u16(ctx.offset(), 2, value as u16);
    }

    #[inline(always)]
    fn stats(&self, ctx: CtxRef) -> StateRef {
        StateRef::at(self.unit_u32(ctx.offset(), 4))
    }

    #[inline(always)]
    fn set_stats(&mut self, ctx: CtxRef, value: StateRef) {
        self.set_unit_u32(ctx.offset(), 4, value.offset());
    }

    #[inline(always)]
    fn suffix(&self, ctx: CtxRef) -> CtxRef {
        CtxRef::at(self.unit_u32(ctx.offset(), 8))
    }

    #[inline(always)]
    fn set_suffix(&mut self, ctx: CtxRef, value: CtxRef) {
        self.set_unit_u32(ctx.offset(), 8, value.offset());
    }

    #[inline(always)]
    fn stamp(&self, node: UnitRef) -> u16 {
        self.unit_u16(node.offset(), 0)
    }

    #[inline(always)]
    fn set_stamp(&mut self, node: UnitRef, value: u16) {
        self.set_unit_u16(node.offset(), 0, value);
    }

    #[inline(always)]
    fn nu(&self, node: UnitRef) -> u32 {
        self.unit_u16(node.offset(), 2) as u32
    }

    #[inline(always)]
    fn set_nu(&mut self, node: UnitRef, value: u32) {
        self.set_unit_u16(node.offset(), 2, value as u16);
    }

    #[inline(always)]
    fn node_next(&self, node: UnitRef) -> UnitRef {
        UnitRef::at(self.unit_u32(node.offset(), 4))
    }

    #[inline(always)]
    fn set_node_next(&mut self, node: UnitRef, value: UnitRef) {
        self.set_unit_u32(node.offset(), 4, value.offset());
    }

    #[inline(always)]
    fn free_link(&self, node: UnitRef) -> UnitRef {
        UnitRef::at(self.unit_u32(node.offset(), 0))
    }

    #[inline(always)]
    fn set_free_link(&mut self, node: UnitRef, value: UnitRef) {
        self.set_unit_u32(node.offset(), 0, value.offset());
    }
}

pub(crate) struct PPMd7<RC> {
    min_context: CtxRef,
    max_context: CtxRef,
    found_state: StateRef,
    order_fall: u32,
    init_esc: u32,
    prev_success: u32,
    max_order: u32,
    hi_bits_flag: u32,
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
    ns2bs_index: [u8; 256],
    ns2index: [u8; 256],
    exp_escape: [u8; 16],
    dummy_see: See,
    see: [[See; 16]; 25],
    bin_summ: [[u16; 64]; 128],
    mem: Memory,
    rc: RC,
}

impl<RC> PPMd7<RC> {
    fn construct(rc: RC, order: u32, mem_size: u32) -> Result<Self, Error> {
        let mut units2index = [0u8; 128];
        let mut index2units = [0u8; 40];

        let mut k = 0;
        for i in 0..PPMD_NUM_INDEXES {
            let step: u32 = if i >= 12 { 4 } else { (i >> 2) + 1 };
            for _ in 0..step {
                units2index[k as usize] = i as u8;
                k += 1;
            }
            index2units[i as usize] = k as u8;
        }

        let mut ns2bs_index = [0u8; 256];
        ns2bs_index[0] = (0 << 1) as u8;
        ns2bs_index[1] = (1 << 1) as u8;
        ns2bs_index[2..11].fill((2 << 1) as u8);
        ns2bs_index[11..256].fill((3 << 1) as u8);

        let mut ns2index = [0u8; 256];
        for i in 0..3 {
            ns2index[i as usize] = i as u8;
        }

        let mut m = 3;
        let mut k = 1;
        for i in 3..256 {
            ns2index[i as usize] = m as u8;
            k -= 1;
            if k == 0 {
                m += 1;
                k = m - 2;
            }
        }

        let align_offset = (4u32.wrapping_sub(mem_size)) & 3;
        let total_size = align_offset as usize + mem_size as usize;
        let mem = Memory::new(total_size)?;

        let mut ppmd = Self {
            min_context: CtxRef::NULL,
            max_context: CtxRef::NULL,
            found_state: StateRef::NULL,
            order_fall: 0,
            init_esc: 0,
            prev_success: 0,
            max_order: order,
            hi_bits_flag: 0,
            run_length: 0,
            init_rl: 0,
            size: mem_size,
            glue_count: 0,
            align_offset,
            lo_unit: 0,
            hi_unit: 0,
            text: 0,
            units_start: 0,
            units2index,
            index2units,
            ns2bs_index,
            ns2index,
            exp_escape: K_EXP_ESCAPE,
            dummy_see: See::default(),
            see: [[See::default(); 16]; 25],
            free_list: [UnitRef::NULL; PPMD_NUM_INDEXES as usize],
            bin_summ: [[0; 64]; 128],
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

    fn insert_node(&mut self, node: UnitRef, indx: u32) {
        self.mem.set_free_link(node, self.free_list[indx as usize]);
        self.free_list[indx as usize] = node;
    }

    /// Removes the first node from a free list. The list must hold at least one node.
    fn remove_node(&mut self, indx: u32) -> UnitRef {
        let node = self.free_list[indx as usize];
        self.free_list[indx as usize] = self.mem.free_link(node);
        node
    }

    fn split_block(&mut self, ptr: UnitRef, old_index: u32, new_index: u32) {
        let nu = (self.index2units[old_index as usize] as u32)
            - (self.index2units[new_index as usize] as u32);
        let ptr = ptr.add_units(self.index2units[new_index as usize] as u32);
        let mut i = self.units2index[(nu as usize) - 1] as u32;

        if self.index2units[i as usize] as u32 != nu {
            i -= 1;
            let k = self.index2units[i as usize] as u32;
            self.insert_node(ptr.add_units(k), nu - k - 1);
        }

        self.insert_node(ptr, i);
    }

    /// We use the first u16 field of the 12-bytes as record type stamp.
    /// State   { symbol: u8, freq: u8, .. : freq != 0
    /// Context { num_stats: u16, ..       : num_stats != 0
    /// Node    { stamp: u16               : stamp == 0 for free record
    ///                                    : stamp == 1 for head record and guard
    /// Last 12-bytes record in array is always containing the 12-bytes order-0 Context
    /// record.
    fn glue_free_blocks(&mut self) {
        let mut n = UnitRef::NULL;
        self.glue_count = 255;

        // We set guard node at lo_unit.
        if self.lo_unit != self.hi_unit {
            self.mem.set_stamp(UnitRef::at(self.lo_unit), 1);
        }

        // Create list of free blocks. We still need one additional list walk pass before glue.
        for i in 0..PPMD_NUM_INDEXES as usize {
            let nu = self.index2units[i] as u32;
            let mut next = self.free_list[i];
            self.free_list[i] = UnitRef::NULL;
            while !next.is_null() {
                let node = next;
                next = self.mem.free_link(node);
                self.mem.set_stamp(node, EMPTY_NODE);
                self.mem.set_nu(node, nu);
                self.mem.set_node_next(node, n);
                n = node;
            }
        }

        // Glue and fill must walk the list in same direction.
        let head = self.glue_blocks(n);
        self.fill_list(head);
    }

    /// Glue free blocks. Returns the new head of the list.
    fn glue_blocks(&mut self, mut n: UnitRef) -> UnitRef {
        let mut head = n;
        // The node whose next field links to the current node. `None` is the list head.
        let mut prev: Option<UnitRef> = None;
        while !n.is_null() {
            let node = n;
            let mut nu = self.mem.nu(node);
            n = self.mem.node_next(node);
            if nu == 0 {
                match prev {
                    None => head = n,
                    Some(prev) => self.mem.set_node_next(prev, n),
                }
            } else {
                prev = Some(node);
                loop {
                    let node2 = node.add_units(nu);
                    nu += self.mem.nu(node2);
                    if self.mem.stamp(node2) != EMPTY_NODE || nu >= 0x10000 {
                        break;
                    }
                    self.mem.set_nu(node, nu);
                    self.mem.set_nu(node2, 0);
                }
            }
        }
        head
    }

    /// Fill lists of free blocks.
    fn fill_list(&mut self, head: UnitRef) {
        let mut n = head;
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
                self.insert_node(node.add_units(k), nu - k - 1);
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

    #[inline(never)]
    fn restart_model(&mut self) {
        self.free_list = [UnitRef::NULL; 38];

        self.text = self.align_offset;
        self.hi_unit = self.text + self.size;
        self.units_start = self.hi_unit - self.size / 8 / UNIT_SIZE * 7 * UNIT_SIZE;
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

        self.hi_unit -= UNIT_SIZE;
        let mc = UnitRef::at(self.hi_unit).as_ctx();
        let s = UnitRef::at(self.lo_unit).as_states();

        self.lo_unit += (256 / 2) * UNIT_SIZE;
        self.min_context = mc;
        self.max_context = mc;
        self.found_state = s;

        self.mem.set_num_stats(mc, 256);
        self.mem.set_summ_freq(mc, 256 + 1);
        self.mem.set_stats(mc, s);
        self.mem.set_suffix(mc, CtxRef::NULL);

        (0..256).for_each(|i| {
            let s = s.add(i);
            self.mem.set_symbol(s, i as u8);
            self.mem.set_freq(s, 1);
            self.mem.set_successor(s, Successor::NULL);
        });

        (0..128).for_each(|i| {
            (0..8).for_each(|k| {
                let val = PPMD_BIN_SCALE - (K_INIT_BIN_ESC[k] as u32) / (i as u32 + 2);

                (0..64).step_by(8).for_each(|m| {
                    self.bin_summ[i][k + m] = val as u16;
                });
            });
        });

        (0..25).for_each(|i| {
            let summ = (5 * i as u32 + 10) << (PPMD_PERIOD_BITS - 4);
            (0..16).for_each(|k| {
                let s = &mut self.see[i][k];
                s.summ = summ as u16;
                s.shift = (PPMD_PERIOD_BITS - 4) as u8;
                s.count = 4;
            });
        });

        self.dummy_see.summ = 0;
        self.dummy_see.shift = PPMD_PERIOD_BITS as u8;
        self.dummy_see.count = 64;
    }

    /// Returns the state with the given symbol in the stats of a context with more than one
    /// state.
    #[inline(always)]
    fn find_state<B: AsRef<[u8]> + AsMut<[u8]>>(
        mem: &Memory<B>,
        ctx: CtxRef,
        symbol: u8,
    ) -> StateRef {
        let mut s = mem.stats(ctx);
        while mem.symbol(s) != symbol {
            s = s.next();
        }
        s
    }

    /// It's called when `found_state.successor` is RAW-successor, that is the link to position
    /// in raw text. So we create Context records and write the links to `found_state.successor`
    /// and to identical RAW-successors in suffix contexts of `min_context`.
    ///
    /// The function returns:
    ///
    /// if (`porder_fall` == 0) then `min_context` is already at MAX order,
    ///   { return pointer to new or existing context of same MAX order }
    /// else
    ///   { return pointer to new real context that will be (order+1) in comparison with `min_context` }
    ///
    /// also it can return a pointer to a real context of same order.
    #[inline(never)]
    fn create_successors(&mut self) -> Option<CtxRef> {
        let mut c = self.min_context;
        let up_branch = self.mem.successor(self.found_state);
        let mut num_ps = 0;
        let mut ps = [StateRef::NULL; 64];

        if self.order_fall != 0 {
            ps[num_ps] = self.found_state;
            num_ps += 1;
        }

        while !self.mem.suffix(c).is_null() {
            c = self.mem.suffix(c);

            let s = if self.mem.num_stats(c) != 1 {
                Self::find_state(&self.mem, c, self.mem.symbol(self.found_state))
            } else {
                c.one_state()
            };
            let successor = self.mem.successor(s);
            if successor != up_branch {
                // c is the real record Context here.
                c = successor.context();
                if num_ps == 0 {
                    // c is the real record MAX Order Context here,
                    // so we don't need to create any new contexts.
                    return Some(c);
                }
                break;
            }
            ps[num_ps] = s;
            num_ps += 1;
        }

        // All created contexts will have single-symbol with new RAW-successor
        // All new RAW-successors will point to next position in RAW text
        // after `found_state.successor`
        let new_sym = self.mem.text(up_branch.text_position());
        let up_branch = Successor::text(up_branch.text_position() + 1);

        let new_freq = if self.mem.num_stats(c) == 1 {
            self.mem.freq(c.one_state())
        } else {
            let s = Self::find_state(&self.mem, c, new_sym);
            let cf = (self.mem.freq(s) as u32) - 1;
            let s0 = self.mem.summ_freq(c) - self.mem.num_stats(c) - cf;

            // cf - is frequency of symbol that will be successor in new context records.
            // s0 - is commutative frequency sum of another symbols from parent context.
            // max(new_freq) = (s.freq + 1), when (s0 == 1)
            // We have a requirement (Context::get_one_state().freq <= 128) in bin_summ
            // so (s.freq < 128) - is a requirement for multi-symbol contexts.
            1 + (if 2 * cf <= s0 {
                (5 * cf > s0) as u32
            } else {
                ((2 * cf + s0 - 1) / (2 * s0)) + 1
            }) as u8
        };

        // Create a new single-symbol contexts from low order to high order in loop.
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

            let mut mem = self.mem.view();
            mem.set_num_stats(c1, 1);

            let state = c1.one_state();
            mem.set_symbol(state, new_sym);
            mem.set_freq(state, new_freq);
            mem.set_successor(state, up_branch);

            mem.set_suffix(c1, c);
            num_ps -= 1;

            mem.set_successor(ps[num_ps], Successor::from(c1));

            c = c1;
            if num_ps == 0 {
                break;
            }
        }

        Some(c)
    }

    #[inline(never)]
    fn update_model(&mut self) {
        let mut mem = self.mem.view();
        let mc = self.min_context;
        let found_freq = mem.freq(self.found_state);
        let found_symbol = mem.symbol(self.found_state);

        if found_freq < MAX_FREQ / 4 && !mem.suffix(mc).is_null() {
            // Update freqs in suffix context
            let c = mem.suffix(mc);

            if mem.num_stats(c) == 1 {
                let s = c.one_state();
                let freq = mem.freq(s);
                if freq < 32 {
                    mem.set_freq(s, freq + 1);
                }
            } else {
                let mut s = mem.stats(c);
                if mem.symbol(s) != found_symbol {
                    s = Self::find_state(&mem, c, found_symbol);
                    if mem.freq(s) >= mem.freq(s.prev()) {
                        mem.swap_states(s, s.prev());
                        s = s.prev();
                    }
                }
                let freq = mem.freq(s);
                if freq < MAX_FREQ - 9 {
                    mem.set_freq(s, freq + 2);
                    mem.set_summ_freq(c, mem.summ_freq(c) + 2);
                }
            }
        }

        if self.order_fall == 0 {
            // MAX ORDER context
            // (found_state.Successor) is RAW-successor.
            match self.create_successors() {
                None => {
                    self.restart_model();
                    return;
                }
                Some(mc) => {
                    self.min_context = mc;
                    self.max_context = mc;
                }
            }

            self.mem
                .set_successor(self.found_state, Successor::from(self.min_context));
            return;
        }

        // NON MAX ORDER context
        self.mem.set_text(self.text, found_symbol);
        self.text += 1;
        if self.text >= self.units_start {
            self.restart_model();
            return;
        }
        let mut max_successor = Successor::text(self.text);

        let mut min_successor = self.mem.successor(self.found_state);

        if min_successor.is_null() {
            // found_state has NULL-successor here.
            // And only root 0-order context can contain NULL-successors.
            // We change successor in found_state to RAW-successor,
            // And next context will be same 0-order root Context.
            self.mem.set_successor(self.found_state, max_successor);
            min_successor = Successor::from(self.min_context);
        } else {
            // There is a successor for found_state in min_context.
            // So the next context will be one order higher than min_context.

            if min_successor <= max_successor {
                // min_successor is RAW-successor. So we will create real contexts records:
                match self.create_successors() {
                    None => {
                        self.restart_model();
                        return;
                    }
                    Some(context) => {
                        min_successor = Successor::from(context);
                    }
                }
            }

            // min_successor now is real Context pointer that points to existing (Order+1) context.

            self.order_fall -= 1;
            if self.order_fall == 0 {
                // If we move to max_order context, then min_successor will be common Successor for both:
                //   min_context that is (max_order - 1)
                //   max_context that is (max_order)
                // so we don't need new RAW-successor, and we can use real min_successor
                // as successors for both min_context and max_context.
                max_successor = min_successor;

                // if (max_context != min_context)
                // {
                //   There was order fall from max_order, and we don't need current symbol
                //   to transfer some RAW-successors to real contexts.
                //   So we roll back pointer in raw data for one position.
                // }
                self.text -= (self.max_context != self.min_context) as u32;
            }
        }

        let mc = self.min_context;
        let mut c = self.max_context;

        self.min_context = min_successor.context();
        self.max_context = self.min_context;

        if c == mc {
            return;
        }

        // s0 : is pure escape freq
        let ns = self.mem.num_stats(mc);
        let found_freq = self.mem.freq(self.found_state) as u32;
        let s0 = self.mem.summ_freq(mc) - ns - (found_freq - 1);

        while c != mc {
            let mut sum;
            let ns1 = self.mem.num_stats(c);
            if ns1 != 1 {
                if ns1 & 1 == 0 {
                    // Expand for one unit
                    let old_nu = ns1 >> 1;
                    let i = self.units2index[(old_nu as usize) - 1] as u32;
                    if i != self.units2index[old_nu as usize] as u32 {
                        let Some(ptr) = self.alloc_units(i + 1) else {
                            self.restart_model();
                            return;
                        };

                        let old_ptr = self.mem.stats(c).as_unit();
                        self.mem.copy_units(old_ptr, ptr, old_nu);
                        self.insert_node(old_ptr, i);
                        self.mem.set_stats(c, ptr.as_states());
                    }
                }
                sum = self.mem.summ_freq(c);
                // Max increase of escape_freq is 3 here.
                // Total increase of union2.summ_freq for all symbols is less than 256 here.
                sum += ((2 * ns1 < ns) as u32)
                    + 2 * ((4 * ns1 <= ns) as u32 & (sum <= 8 * ns1) as u32);
            } else {
                // Instead of 1-symbol context we create 2-symbol context.
                let Some(s) = self.alloc_units(0) else {
                    self.restart_model();
                    return;
                };
                let s = s.as_states();

                let mut mem = self.mem.view();
                let one_state = c.one_state();
                let mut freq = mem.freq(one_state) as u32;
                mem.set_symbol(s, mem.symbol(one_state));
                mem.set_successor(s, mem.successor(one_state));
                mem.set_stats(c, s);
                if freq < (MAX_FREQ / 4 - 1) as u32 {
                    freq <<= 1;
                } else {
                    freq = (MAX_FREQ - 4) as u32;
                }
                // (max(s.freq) == 120), when we convert from 1-symbol into 2-symbol context.
                mem.set_freq(s, freq as u8);
                // max(init_esc = K_EXP_ESCAPE[*]) is 25. So the max(escape_freq) is 26 here.
                sum = freq + self.init_esc + ((ns > 3) as u32);
            }

            let mut mem = self.mem.view();
            let s = mem.stats(c).add(ns1);
            let mut cf = 2 * (sum + 6) * found_freq;
            let sf = s0 + sum;
            mem.set_symbol(s, found_symbol);
            mem.set_num_stats(c, ns1 + 1);

            mem.set_successor(s, max_successor);
            if cf < 6 * sf {
                cf = 1 + ((cf > sf) as u32) + ((cf >= 4 * sf) as u32);
                sum += 3;
                // It can add (0, 1, 2) to escape_freq
            } else {
                cf = 4
                    + ((cf >= 9 * sf) as u32)
                    + ((cf >= 12 * sf) as u32)
                    + ((cf >= 15 * sf) as u32);
                sum += cf;
            }

            mem.set_summ_freq(c, sum);
            mem.set_freq(s, cf as u8);

            c = mem.suffix(c);
        }
    }

    #[inline(never)]
    fn rescale(&mut self) {
        let mut mem = self.mem.view();
        let mc = self.min_context;
        let stats = mem.stats(mc);
        let mut s = self.found_state;

        // Sort the list by freq
        if s != stats {
            let tmp = mem.state(s);
            while s != stats {
                mem.copy_state(s.prev(), s);
                s = s.prev();
            }
            mem.set_state(s, tmp);
        }

        let mut sum_freq = mem.freq(s) as u32;
        let mut esc_freq = mem.summ_freq(mc) - sum_freq;

        // if (p.order_fall == 0), adder = 0 : it's     allowed to remove symbol from     MAX order context
        // if (p.order_fall != 0), adder = 1 : it's NOT allowed to remove symbol from NON-MAX order context

        let adder = (self.order_fall != 0) as u32;

        sum_freq = (sum_freq + 4 + adder) >> 1;
        let mut i = mem.num_stats(mc) - 1;
        mem.set_freq(s, sum_freq as u8);

        for _ in 0..i {
            s = s.next();
            let mut freq = mem.freq(s) as u32;
            esc_freq -= freq;
            freq = (freq + adder) >> 1;
            sum_freq += freq;
            mem.set_freq(s, freq as u8);
            if freq > mem.freq(s.prev()) as u32 {
                let tmp = mem.state(s);
                let mut s1 = s;
                loop {
                    mem.copy_state(s1.prev(), s1);
                    s1 = s1.prev();
                    if !(s1 != stats && freq > mem.freq(s1.prev()) as u32) {
                        break;
                    }
                }
                mem.set_state(s1, tmp);
            }
        }

        if mem.freq(s) == 0 {
            // Remove all items with freq == 0

            i = 0;
            while mem.freq(s) == 0 {
                i += 1;
                s = s.prev();
            }

            // We increase (esc_freq) for the number of removed symbols.
            // So we will have (0.5) increase for escape_freq in average per
            // removed symbol after escape_freq halving
            esc_freq += i;
            let num_stats = mem.num_stats(mc);
            let num_stats_new = num_stats.wrapping_sub(i);
            mem.set_num_stats(mc, num_stats_new);
            let n0 = (num_stats + 1) >> 1;

            if num_stats_new == 1 {
                // Create Single-Symbol context
                let mut freq = mem.freq(stats) as u32;
                loop {
                    esc_freq >>= 1;
                    freq = (freq + 1) >> 1;
                    if esc_freq <= 1 {
                        break;
                    }
                }

                let s = mc.one_state();
                mem.copy_state(stats, s);
                mem.set_freq(s, freq as u8); // (freq <= 260 / 4)
                self.found_state = s;
                self.insert_node(stats.as_unit(), self.units2index[(n0 as usize) - 1] as u32);
                return;
            }

            let n1 = (num_stats_new + 1) >> 1;
            if n0 != n1 {
                let i0 = self.units2index[(n0 as usize) - 1] as u32;
                let i1 = self.units2index[(n1 as usize) - 1] as u32;
                if i0 != i1 {
                    if !self.free_list[i1 as usize].is_null() {
                        let ptr = self.remove_node(i1);
                        self.mem.set_stats(mc, ptr.as_states());
                        self.mem.copy_units(stats.as_unit(), ptr, n1);
                        self.insert_node(stats.as_unit(), i0);
                    } else {
                        self.split_block(stats.as_unit(), i0, i1);
                    }
                }
            }
        }

        // escape_freq halving here.
        self.mem
            .set_summ_freq(mc, sum_freq + esc_freq - (esc_freq >> 1));
        self.found_state = self.mem.stats(mc);
    }

    fn make_esc_freq(&mut self, num_masked: u32, esc_freq: &mut u32) -> SeeSource {
        let num_stats = self.mem.num_stats(self.min_context);

        if num_stats != 256 {
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

    fn get_see(&mut self, see_source: SeeSource) -> &mut See {
        match see_source {
            SeeSource::Dummy => &mut self.dummy_see,
            SeeSource::Table(i, k) => &mut self.see[i][k],
        }
    }

    fn calculate_see_table_hash(&self, num_masked: u32, num_stats: u32) -> (usize, usize) {
        let non_masked = num_stats - num_masked;
        let base_context_idx = self.ns2index[(non_masked as usize) - 1] as usize;

        let mc = self.min_context;
        let suffix_num_stats = self.mem.num_stats(self.mem.suffix(mc));
        let summ_freq = self.mem.summ_freq(mc);

        let context_hierarchy_hash = (non_masked < (suffix_num_stats - num_stats)) as usize;
        let freq_distribution_hash = 2 * (summ_freq < (11 * num_stats)) as usize;
        let symbol_masking_ratio_hash = 4 * (num_masked > non_masked) as usize;
        let symbol_characteristics_hash = self.hi_bits_flag as usize;

        let see_table_hash = context_hierarchy_hash
            + freq_distribution_hash
            + symbol_masking_ratio_hash
            + symbol_characteristics_hash;

        (base_context_idx, see_table_hash)
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
        let mut mem = self.mem.view();
        let mut s = self.found_state;
        let freq = mem.freq(s) as u32 + 4;
        let mc = self.min_context;
        mem.set_summ_freq(mc, mem.summ_freq(mc) + 4);
        mem.set_freq(s, freq as u8);
        if freq > mem.freq(s.prev()) as u32 {
            mem.swap_states(s, s.prev());
            s = s.prev();
            self.found_state = s;
            if freq > MAX_FREQ as u32 {
                self.rescale();
            }
        }
        self.next_context();
    }

    fn update1_0(&mut self) {
        let mut mem = self.mem.view();
        let s = self.found_state;
        let mc = self.min_context;
        let mut freq = mem.freq(s) as u32;
        let summ_freq = mem.summ_freq(mc);
        self.prev_success = ((2 * freq) > summ_freq) as u32;
        self.run_length += self.prev_success as i32;
        mem.set_summ_freq(mc, summ_freq + 4);
        freq += 4;
        mem.set_freq(s, freq as u8);
        if freq > MAX_FREQ as u32 {
            self.rescale();
        }
        self.next_context();
    }

    fn update2(&mut self) {
        let mut mem = self.mem.view();
        let s = self.found_state;
        let mc = self.min_context;
        let freq = mem.freq(s) as u32 + 4;
        self.run_length = self.init_rl;
        mem.set_summ_freq(mc, mem.summ_freq(mc) + 4);
        mem.set_freq(s, freq as u8);
        if freq > MAX_FREQ as u32 {
            self.rescale();
        }
        self.update_model();
    }

    fn update_bin(&mut self, s: StateRef) {
        let mut mem = self.mem.view();
        let freq = mem.freq(s);
        self.found_state = s;
        self.prev_success = 1;
        self.run_length += 1;
        mem.set_freq(s, freq + (freq < 128) as u8);
        self.next_context();
    }

    fn mask_symbols(&self, char_mask: &mut [u8; 256], stats: StateRef, num_stats: u32) {
        for state in self
            .mem
            .states(stats, num_stats)
            .chunks_exact(STATE_SIZE as usize)
        {
            char_mask[state[0] as usize] = 0;
        }
    }

    const fn hi_bits_flag3(symbol: u32) -> u32 {
        (symbol + 0xC0) >> (8 - 3) & (1 << 3)
    }

    const fn hi_bits_flag4(symbol: u32) -> u32 {
        (symbol + 0xC0) >> (8 - 4) & (1 << 4)
    }

    fn get_bin_summ(&mut self) -> &mut u16 {
        let state = self.min_context.one_state();

        let hi_bits_flag3 = Self::hi_bits_flag3(self.mem.symbol(self.found_state) as u32);
        let symbol = self.mem.symbol(state) as u32;
        let hi_bits_flag4 = Self::hi_bits_flag4(symbol);

        self.hi_bits_flag = hi_bits_flag3;

        let freq_bin_idx = self.mem.freq(state) as usize;

        let num_stats = self.mem.num_stats(self.mem.suffix(self.min_context)) as usize;

        let context_idx = (self.prev_success
            + ((self.run_length as u32 >> 26) & 0x20)
            + self.ns2bs_index[num_stats - 1] as u32
            + hi_bits_flag4
            + hi_bits_flag3) as usize;

        &mut self.bin_summ[freq_bin_idx - 1][context_idx]
    }
}

impl<R: Read> PPMd7<RangeDecoder<R>> {
    pub(crate) fn new_decoder(
        reader: R,
        order: u32,
        mem_size: u32,
    ) -> Result<PPMd7<RangeDecoder<R>>, Error> {
        let range_decoder = RangeDecoder::new(reader)?;
        Self::construct(range_decoder, order, mem_size)
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

impl<R: Read> PPMd7<RangeDecoder7a<R>> {
    pub(crate) fn new_decoder_7a(
        reader: R,
        order: u32,
        mem_size: u32,
    ) -> Result<PPMd7<RangeDecoder7a<R>>, Error> {
        let range_decoder = RangeDecoder7a::new(reader)?;
        Self::construct(range_decoder, order, mem_size)
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

impl<W: Write> PPMd7<RangeEncoder<W>> {
    pub(crate) fn new_encoder(
        writer: W,
        order: u32,
        mem_size: u32,
    ) -> Result<PPMd7<RangeEncoder<W>>, Error> {
        let range_encoder = RangeEncoder::new(writer);
        Self::construct(range_encoder, order, mem_size)
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

impl<W: Write> PPMd7<RangeEncoder7a<W>> {
    pub(crate) fn new_encoder_7a(
        writer: W,
        order: u32,
        mem_size: u32,
    ) -> Result<PPMd7<RangeEncoder7a<W>>, Error> {
        let range_encoder = RangeEncoder7a::new(writer);
        Self::construct(range_encoder, order, mem_size)
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

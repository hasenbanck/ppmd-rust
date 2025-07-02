mod decoder;
mod encoder;
mod range_coding;

use std::{
    alloc::{alloc_zeroed, dealloc, Layout},
    io::{Read, Write},
    mem::ManuallyDrop,
    ptr::{addr_of_mut, NonNull},
};

pub(crate) use range_coding::{RangeDecoder, RangeEncoder};

use super::*;
use crate::Error;

const MAX_FREQ: u8 = 124;
const UNIT_SIZE: isize = 12;
const K_TOP_VALUE: u32 = 1 << 24;
const EMPTY_NODE: u16 = 0;

static K_EXP_ESCAPE: [u8; 16] = [25, 14, 9, 7, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 2, 2];

static K_INIT_BIN_ESC: [u16; 8] = [
    0x3CDD, 0x1F3F, 0x59BF, 0x48F3, 0x64A1, 0x5ABC, 0x6632, 0x6051,
];

#[derive(Copy, Clone)]
#[repr(C)]
struct Node {
    stamp: u16,
    nu: u16,
    next: u32,
    prev: u32,
}

#[derive(Copy, Clone)]
#[repr(C)]
union NodeUnion {
    node: Node,
    next_ref: u32,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct Context {
    num_stats: u16,
    union2: Union2,
    union4: Union4,
    suffix: u32,
}

pub(crate) struct PPMd7<RC> {
    min_context: NonNull<Context>,
    max_context: NonNull<Context>,
    found_state: NonNull<State>,
    order_fall: std::ffi::c_uint,
    init_esc: std::ffi::c_uint,
    prev_success: std::ffi::c_uint,
    max_order: std::ffi::c_uint,
    hi_bits_flag: std::ffi::c_uint,
    run_length: i32,
    init_rl: i32,
    size: u32,
    glue_count: u32,
    align_offset: u32,
    lo_unit: NonNull<u8>,
    hi_unit: NonNull<u8>,
    text: NonNull<u8>,
    units_start: NonNull<u8>,
    index2units: [u8; 40],
    units2index: [u8; 128],
    free_list: [u32; 38],
    ns2bs_index: [u8; 256],
    ns2index: [u8; 256],
    exp_escape: [u8; 16],
    dummy_see: See,
    see: [[See; 16]; 25],
    bin_summ: [[u16; 64]; 128],
    memory_ptr: NonNull<u8>,
    memory_layout: Layout,
    rc: RC,
}

impl<RC> Drop for PPMd7<RC> {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.memory_ptr.as_ptr(), self.memory_layout);
        }
    }
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
        let total_size = (align_offset + mem_size) as usize;

        let memory_layout = Layout::from_size_align(total_size, align_of::<usize>())
            .expect("Failed to create memory layout");

        let allocation = unsafe {
            assert_ne!(total_size, 0);
            NonNull::new(alloc_zeroed(memory_layout))
        };

        let Some(memory_ptr) = allocation else {
            return Err(Error::MemoryAllocation);
        };

        let mut ppmd = Self {
            min_context: NonNull::dangling(),
            max_context: NonNull::dangling(),
            found_state: NonNull::dangling(),
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
            lo_unit: NonNull::dangling(),
            hi_unit: NonNull::dangling(),
            text: NonNull::dangling(),
            units_start: NonNull::dangling(),
            units2index,
            index2units,
            ns2bs_index,
            ns2index,
            exp_escape: K_EXP_ESCAPE,
            dummy_see: See::default(),
            see: [[See::default(); 16]; 25],
            free_list: [0; PPMD_NUM_INDEXES as usize],
            bin_summ: [[0; 64]; 128],
            memory_ptr,
            memory_layout,
            rc,
        };

        unsafe { ppmd.restart_model() };

        Ok(ppmd)
    }

    unsafe fn insert_node(&mut self, node: NonNull<u8>, indx: std::ffi::c_uint) {
        *node.cast::<u32>().as_mut() = self.free_list[indx as usize];
        self.free_list[indx as usize] =
            node.offset_from(self.memory_ptr) as std::ffi::c_long as u32;
    }

    unsafe fn remove_node(&mut self, indx: std::ffi::c_uint) -> NonNull<u8> {
        let node = self
            .memory_ptr
            .offset(self.free_list[indx as usize] as isize)
            .cast::<u32>();
        self.free_list[indx as usize] = *node.as_ref();
        node.cast()
    }

    unsafe fn split_block(
        &mut self,
        mut ptr: NonNull<u8>,
        oldIndx: std::ffi::c_uint,
        newIndx: std::ffi::c_uint,
    ) {
        let mut i: std::ffi::c_uint = 0;
        let nu: std::ffi::c_uint = (self.index2units[oldIndx as usize] as std::ffi::c_uint)
            .wrapping_sub(self.index2units[newIndx as usize] as std::ffi::c_uint);
        ptr = ptr.offset(
            (self.index2units[newIndx as usize] as std::ffi::c_uint * 12 as std::ffi::c_int as u32)
                as isize,
        );
        i = self.units2index[(nu as usize).wrapping_sub(1 as std::ffi::c_int as usize)]
            as std::ffi::c_uint;
        if self.index2units[i as usize] as std::ffi::c_uint != nu {
            i = i.wrapping_sub(1);
            let k: std::ffi::c_uint = self.index2units[i as usize] as std::ffi::c_uint;
            self.insert_node(
                ptr.offset((k * 12 as std::ffi::c_int as u32) as isize),
                nu.wrapping_sub(k)
                    .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint),
            );
        }
        self.insert_node(ptr, i);
    }

    unsafe fn glue_free_blocks(&mut self) {
        let mut head: u32 = 0;
        let mut n: u32 = 0 as std::ffi::c_int as u32;
        self.glue_count = 255 as std::ffi::c_int as u32;
        if self.lo_unit != self.hi_unit {
            self.lo_unit.cast::<Node>().as_mut().stamp = 1;
        }
        let mut i: std::ffi::c_uint = 0;
        i = 0 as std::ffi::c_int as std::ffi::c_uint;
        while i
            < (4 as std::ffi::c_int
                + 4 as std::ffi::c_int
                + 4 as std::ffi::c_int
                + (128 as std::ffi::c_int + 3 as std::ffi::c_int
                    - 1 as std::ffi::c_int * 4 as std::ffi::c_int
                    - 2 as std::ffi::c_int * 4 as std::ffi::c_int
                    - 3 as std::ffi::c_int * 4 as std::ffi::c_int)
                    / 4 as std::ffi::c_int) as std::ffi::c_uint
        {
            let nu: u16 = self.index2units[i as usize] as u16;
            let mut next: u32 = self.free_list[i as usize];
            self.free_list[i as usize] = 0 as std::ffi::c_int as u32;
            while next != 0 as std::ffi::c_int as u32 {
                let mut un = (self.memory_ptr)
                    .offset(next as isize)
                    .cast::<Node>()
                    .cast::<NodeUnion>();
                let tmp: u32 = next;
                next = un.as_ref().next_ref;
                un.as_mut().node.stamp = 0 as std::ffi::c_int as u16;
                un.as_mut().node.nu = nu;
                un.as_mut().node.next = n;
                n = tmp;
            }
            i = i.wrapping_add(1);
            i;
        }
        head = n;
        let mut prev = &mut head;
        while n != 0 {
            let mut node = self.memory_ptr.offset(n as isize).cast::<Node>();
            let mut nu_0 = node.as_ref().nu as u32;
            n = node.as_ref().next;
            if nu_0 == 0 as std::ffi::c_int as u32 {
                *prev = n;
            } else {
                prev = &mut node.as_mut().next;
                loop {
                    let mut node2 = node.offset(nu_0 as isize);
                    nu_0 = nu_0.wrapping_add(node2.as_ref().nu as u32);
                    if node2.as_ref().stamp as std::ffi::c_int != 0 as std::ffi::c_int
                        || nu_0 >= 0x10000 as std::ffi::c_int as u32
                    {
                        break;
                    }
                    node.as_mut().nu = nu_0 as u16;
                    node2.as_mut().nu = 0;
                }
            }
        }
        n = head;
        while n != 0 as std::ffi::c_int as u32 {
            let mut node_0 = self.memory_ptr.offset(n as isize).cast::<Node>();
            let mut nu_1: u32 = node_0.as_ref().nu as u32;
            let mut i_0: std::ffi::c_uint = 0;
            n = node_0.as_ref().next;
            if nu_1 == 0 as std::ffi::c_int as u32 {
                continue;
            }
            while nu_1 > 128 as std::ffi::c_int as u32 {
                self.insert_node(
                    node_0.cast(),
                    (4 as std::ffi::c_int
                        + 4 as std::ffi::c_int
                        + 4 as std::ffi::c_int
                        + (128 as std::ffi::c_int + 3 as std::ffi::c_int
                            - 1 as std::ffi::c_int * 4 as std::ffi::c_int
                            - 2 as std::ffi::c_int * 4 as std::ffi::c_int
                            - 3 as std::ffi::c_int * 4 as std::ffi::c_int)
                            / 4 as std::ffi::c_int
                        - 1 as std::ffi::c_int) as std::ffi::c_uint,
                );
                nu_1 = nu_1.wrapping_sub(128 as std::ffi::c_int as u32);
                node_0 = node_0.offset(128 as std::ffi::c_int as isize);
            }
            i_0 = self.units2index[(nu_1 as usize).wrapping_sub(1 as std::ffi::c_int as usize)]
                as std::ffi::c_uint;
            if self.index2units[i_0 as usize] as std::ffi::c_uint != nu_1 {
                i_0 = i_0.wrapping_sub(1);
                let k: std::ffi::c_uint = self.index2units[i_0 as usize] as std::ffi::c_uint;
                self.insert_node(
                    node_0.offset(k as isize).cast(),
                    nu_1.wrapping_sub(k)
                        .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint),
                );
            }
            self.insert_node(node_0.cast(), i_0);
        }
    }

    #[inline(never)]
    unsafe fn alloc_units_rare(&mut self, indx: std::ffi::c_uint) -> Option<NonNull<u8>> {
        let mut i: std::ffi::c_uint = 0;
        if self.glue_count == 0 as std::ffi::c_int as u32 {
            self.glue_free_blocks();
            if self.free_list[indx as usize] != 0 as std::ffi::c_int as u32 {
                return Some(self.remove_node(indx));
            }
        }
        i = indx;
        loop {
            i = i.wrapping_add(1);
            if i == (4 as std::ffi::c_int
                + 4 as std::ffi::c_int
                + 4 as std::ffi::c_int
                + (128 as std::ffi::c_int + 3 as std::ffi::c_int
                    - 1 as std::ffi::c_int * 4 as std::ffi::c_int
                    - 2 as std::ffi::c_int * 4 as std::ffi::c_int
                    - 3 as std::ffi::c_int * 4 as std::ffi::c_int)
                    / 4 as std::ffi::c_int) as std::ffi::c_uint
            {
                let numBytes: u32 = self.index2units[indx as usize] as std::ffi::c_uint
                    * 12 as std::ffi::c_int as u32;
                let us: NonNull<u8> = self.units_start;
                self.glue_count = (self.glue_count).wrapping_sub(1);
                self.glue_count;
                return if us.offset_from(self.text) as std::ffi::c_long as u32 > numBytes {
                    self.units_start = us.offset(-(numBytes as isize));
                    Some(self.units_start)
                } else {
                    None
                };
            }
            if !(self.free_list[i as usize] == 0 as std::ffi::c_int as u32) {
                break;
            }
        }
        let block = self.remove_node(i);
        self.split_block(block, i, indx);
        Some(block)
    }

    unsafe fn alloc_units(&mut self, indx: std::ffi::c_uint) -> Option<NonNull<u8>> {
        if self.free_list[indx as usize] != 0 as std::ffi::c_int as u32 {
            return Some(self.remove_node(indx));
        }
        let numBytes: u32 =
            self.index2units[indx as usize] as std::ffi::c_uint * 12 as std::ffi::c_int as u32;
        let lo: NonNull<u8> = self.lo_unit;
        if self.hi_unit.offset_from(lo) as std::ffi::c_long as u32 >= numBytes {
            self.lo_unit = lo.offset(numBytes as isize);
            return Some(lo);
        }
        self.alloc_units_rare(indx)
    }

    #[inline(never)]
    unsafe fn restart_model(&mut self) {
        let mut i: std::ffi::c_uint = 0;
        let mut k: std::ffi::c_uint = 0;
        self.free_list.as_mut_ptr().write_bytes(0, 38);
        self.text = self.memory_ptr.offset(self.align_offset as isize);
        self.hi_unit = self.text.offset(self.size as isize);
        self.units_start = self.hi_unit.offset(
            -((self.size / 8 as std::ffi::c_int as u32 / 12 as std::ffi::c_int as u32
                * 7 as std::ffi::c_int as u32
                * 12 as std::ffi::c_int as u32) as isize),
        );
        self.lo_unit = self.units_start;
        self.glue_count = 0 as std::ffi::c_int as u32;
        self.order_fall = self.max_order;
        self.init_rl = -((if self.max_order < 12 as std::ffi::c_int as std::ffi::c_uint {
            self.max_order
        } else {
            12
        }) as i32)
            - 1;
        self.run_length = self.init_rl;
        self.prev_success = 0 as std::ffi::c_int as std::ffi::c_uint;
        self.hi_unit = self.hi_unit.offset(-12);
        let mut mc = self.hi_unit.cast::<Context>();
        let mut s = self.lo_unit.cast::<State>();
        self.lo_unit = (self.lo_unit).offset(
            ((256 as std::ffi::c_int / 2 as std::ffi::c_int) as u32 * 12 as std::ffi::c_int as u32)
                as isize,
        );
        self.min_context = mc;
        self.max_context = self.min_context;
        self.found_state = s;
        mc.as_mut().num_stats = 256 as std::ffi::c_int as u16;
        mc.as_mut().union2.summ_freq = (256 as std::ffi::c_int + 1 as std::ffi::c_int) as u16;
        mc.as_mut().union4.stats = s.cast().offset_from(self.memory_ptr) as std::ffi::c_long as u32;
        mc.as_mut().suffix = 0 as std::ffi::c_int as u32;
        i = 0 as std::ffi::c_int as std::ffi::c_uint;
        while i < 256 as std::ffi::c_int as std::ffi::c_uint {
            s.as_mut().symbol = i as u8;
            s.as_mut().freq = 1;
            s.as_mut().set_successor(0 as std::ffi::c_int as u32);
            i = i.wrapping_add(1);
            s = s.offset(1);
        }
        i = 0 as std::ffi::c_int as std::ffi::c_uint;
        while i < 128 as std::ffi::c_int as std::ffi::c_uint {
            k = 0 as std::ffi::c_int as std::ffi::c_uint;
            while k < 8 as std::ffi::c_int as std::ffi::c_uint {
                let mut m: std::ffi::c_uint = 0;
                let dest = self.bin_summ[i as usize].as_mut_ptr().offset(k as isize);
                let val: u16 = (((1 as std::ffi::c_int)
                    << 7 as std::ffi::c_int + 7 as std::ffi::c_int)
                    as std::ffi::c_uint)
                    .wrapping_sub(
                        (K_INIT_BIN_ESC[k as usize] as std::ffi::c_uint)
                            .wrapping_div(i.wrapping_add(2 as std::ffi::c_int as std::ffi::c_uint)),
                    ) as u16;
                m = 0 as std::ffi::c_int as std::ffi::c_uint;
                while m < 64 as std::ffi::c_int as std::ffi::c_uint {
                    *dest.offset(m as isize) = val;
                    m = m.wrapping_add(8 as std::ffi::c_int as std::ffi::c_uint);
                }
                k = k.wrapping_add(1);
            }
            i = i.wrapping_add(1);
        }
        i = 0 as std::ffi::c_int as std::ffi::c_uint;
        while i < 25 as std::ffi::c_int as std::ffi::c_uint {
            let mut s_0 = self.see[i as usize].as_mut_ptr();
            let summ: std::ffi::c_uint = (5 as std::ffi::c_int as std::ffi::c_uint)
                .wrapping_mul(i)
                .wrapping_add(10 as std::ffi::c_int as std::ffi::c_uint)
                << 7 as std::ffi::c_int - 4 as std::ffi::c_int;
            k = 0 as std::ffi::c_int as std::ffi::c_uint;
            while k < 16 as std::ffi::c_int as std::ffi::c_uint {
                (*s_0).summ = summ as u16;
                (*s_0).shift = (7 as std::ffi::c_int - 4 as std::ffi::c_int) as u8;
                (*s_0).count = 4;
                k = k.wrapping_add(1);
                s_0 = s_0.offset(1);
            }
            i = i.wrapping_add(1);
        }
        self.dummy_see.summ = 0;
        self.dummy_see.shift = 7;
        self.dummy_see.count = 64;
    }

    #[inline(never)]
    unsafe fn create_successors(&mut self) -> Option<NonNull<Context>> {
        let mut c = self.min_context;
        let mut up_branch: u32 = self.found_state.as_ref().successor_0 as u32
            | (self.found_state.as_ref().successor_1 as u32) << 16 as std::ffi::c_int;
        let mut new_sym: u8 = 0;
        let mut new_freq: u8 = 0;
        let mut num_ps: std::ffi::c_uint = 0 as std::ffi::c_int as std::ffi::c_uint;
        let mut ps: [Option<NonNull<State>>; 64] = [None; 64];
        if self.order_fall != 0 as std::ffi::c_int as std::ffi::c_uint {
            let fresh1 = num_ps;
            num_ps = num_ps.wrapping_add(1);
            ps[fresh1 as usize] = Some(self.found_state);
        }
        while c.as_ref().suffix != 0 {
            let mut successor: u32 = 0;
            let mut s;
            c = self
                .memory_ptr
                .offset(c.as_ref().suffix as isize)
                .cast::<Context>();
            if c.as_ref().num_stats as std::ffi::c_int != 1 {
                let sym: u8 = self.found_state.as_ref().symbol;
                s = self
                    .memory_ptr
                    .offset(c.as_ref().union4.stats as isize)
                    .cast::<State>();
                while s.as_ref().symbol as std::ffi::c_int != sym as std::ffi::c_int {
                    s = s.offset(1);
                }
            } else {
                s = NonNull::new_unchecked(addr_of_mut!(c.as_mut().union2)).cast::<State>();
            }
            successor = s.as_ref().successor_0 as u32
                | (s.as_ref().successor_1 as u32) << 16 as std::ffi::c_int;
            if successor != up_branch {
                c = self.memory_ptr.offset(successor as isize).cast::<Context>();
                if num_ps == 0 {
                    return Some(c);
                }
                break;
            } else {
                let fresh2 = num_ps;
                num_ps = num_ps.wrapping_add(1);
                ps[fresh2 as usize] = Some(s);
            }
        }
        new_sym = *self.memory_ptr.offset(up_branch as isize).as_ref();
        up_branch = up_branch.wrapping_add(1);
        if c.as_ref().num_stats as std::ffi::c_int == 1 as std::ffi::c_int {
            new_freq = (*(&mut c.as_mut().union2 as *mut Union2 as *mut State)).freq;
        } else {
            let mut cf: u32 = 0;
            let mut s0: u32 = 0;
            let mut s_0 = self
                .memory_ptr
                .offset(c.as_ref().union4.stats as isize)
                .cast::<State>();
            while s_0.as_ref().symbol as std::ffi::c_int != new_sym as std::ffi::c_int {
                s_0 = s_0.offset(1);
            }
            cf = (s_0.as_ref().freq as u32).wrapping_sub(1 as std::ffi::c_int as u32);
            s0 = (c.as_ref().union2.summ_freq as u32)
                .wrapping_sub(c.as_ref().num_stats as u32)
                .wrapping_sub(cf);
            new_freq = (1 as std::ffi::c_int as u32).wrapping_add(
                if 2 as std::ffi::c_int as u32 * cf <= s0 {
                    (5 as std::ffi::c_int as u32 * cf > s0) as std::ffi::c_int as u32
                } else {
                    ((2 as std::ffi::c_int as u32 * cf)
                        .wrapping_add(s0)
                        .wrapping_sub(1 as std::ffi::c_int as u32)
                        / (2 as std::ffi::c_int as u32 * s0))
                        .wrapping_add(1 as std::ffi::c_int as u32)
                },
            ) as u8;
        }
        loop {
            let mut c1: NonNull<Context>;
            if self.hi_unit != self.lo_unit {
                self.hi_unit = self.hi_unit.offset(-12);
                c1 = self.hi_unit.cast();
            } else if self.free_list[0 as std::ffi::c_int as usize] != 0 as std::ffi::c_int as u32 {
                c1 = self.remove_node(0).cast();
            } else {
                c1 = self.alloc_units_rare(0)?.cast();
            }
            c1.as_mut().num_stats = 1 as std::ffi::c_int as u16;
            let mut state =
                NonNull::new_unchecked(addr_of_mut!(c1.as_mut().union2)).cast::<State>();
            state.as_mut().symbol = new_sym;
            state.as_mut().freq = new_freq;
            state.as_mut().set_successor(up_branch);
            c1.as_mut().suffix = c.cast().offset_from(self.memory_ptr) as std::ffi::c_long as u32;
            num_ps = num_ps.wrapping_sub(1);

            ps[num_ps as usize]
                .expect("ps[num_ps] not set")
                .as_mut()
                .set_successor(c1.cast().offset_from(self.memory_ptr) as std::ffi::c_long as u32);
            c = c1;
            if !(num_ps != 0) {
                break;
            }
        }

        Some(c)
    }

    #[inline(never)]
    pub unsafe fn update_model(&mut self) {
        let mut maxSuccessor: u32 = 0;
        let mut minSuccessor: u32 = 0;
        let mut c: NonNull<Context>;
        let mut mc: NonNull<Context>;
        let mut s0: std::ffi::c_uint = 0;
        let mut ns: std::ffi::c_uint = 0;
        if (self.found_state.as_ref().freq as std::ffi::c_int)
            < 124 as std::ffi::c_int / 4 as std::ffi::c_int
            && self.min_context.as_ref().suffix != 0 as std::ffi::c_int as u32
        {
            c = self
                .memory_ptr
                .offset(self.min_context.as_ref().suffix as isize)
                .cast();
            if c.as_ref().num_stats as std::ffi::c_int == 1 as std::ffi::c_int {
                let mut s = NonNull::new_unchecked(addr_of_mut!(c.as_mut().union2)).cast::<State>();
                if (s.as_ref().freq as std::ffi::c_int) < 32 as std::ffi::c_int {
                    s.as_mut().freq = (s.as_ref().freq).wrapping_add(1);
                    s.as_ref().freq;
                }
            } else {
                let mut s_0 = self
                    .memory_ptr
                    .offset(c.as_ref().union4.stats as isize)
                    .cast::<State>();
                let sym: u8 = self.found_state.as_ref().symbol;
                if s_0.as_ref().symbol as std::ffi::c_int != sym as std::ffi::c_int {
                    loop {
                        s_0 = s_0.offset(1);
                        if !(s_0.as_ref().symbol as std::ffi::c_int != sym as std::ffi::c_int) {
                            break;
                        }
                    }
                    if s_0.offset(0 as std::ffi::c_int as isize).as_ref().freq as std::ffi::c_int
                        >= s_0.offset(-(1 as std::ffi::c_int) as isize).as_ref().freq
                            as std::ffi::c_int
                    {
                        let tmp: State = *s_0.offset(0 as std::ffi::c_int as isize).as_ref();
                        *s_0.offset(0 as std::ffi::c_int as isize).as_mut() =
                            *s_0.offset(-(1 as std::ffi::c_int) as isize).as_ref();
                        *s_0.offset(-(1 as std::ffi::c_int) as isize).as_mut() = tmp;
                        s_0 = s_0.offset(-1);
                    }
                }
                if (s_0.as_ref().freq as std::ffi::c_int)
                    < 124 as std::ffi::c_int - 9 as std::ffi::c_int
                {
                    s_0.as_mut().freq =
                        (s_0.as_ref().freq as std::ffi::c_int + 2 as std::ffi::c_int) as u8;
                    c.as_mut().union2.summ_freq = (c.as_ref().union2.summ_freq as std::ffi::c_int
                        + 2 as std::ffi::c_int)
                        as u16;
                }
            }
        }
        if self.order_fall == 0 as std::ffi::c_int as std::ffi::c_uint {
            let Some(context) = self.create_successors() else {
                self.restart_model();
                return;
            };

            self.min_context = context;
            self.max_context = self.min_context;

            self.found_state.as_mut().set_successor(
                self.min_context.cast().offset_from(self.memory_ptr) as std::ffi::c_long as u32,
            );
            return;
        }
        let mut text: NonNull<u8> = self.text;
        let mut fresh3 = text;
        text = text.offset(1);
        *fresh3.as_mut() = self.found_state.as_ref().symbol;
        self.text = text;
        if text >= self.units_start {
            self.restart_model();
            return;
        }
        maxSuccessor = text.offset_from(self.memory_ptr) as std::ffi::c_long as u32;
        minSuccessor = self.found_state.as_ref().successor_0 as u32
            | (self.found_state.as_ref().successor_1 as u32) << 16 as std::ffi::c_int;
        if minSuccessor != 0 {
            if minSuccessor <= maxSuccessor {
                let Some(cs) = self.create_successors() else {
                    self.restart_model();
                    return;
                };

                minSuccessor = cs.cast().offset_from(self.memory_ptr) as std::ffi::c_long as u32;
            }
            self.order_fall = self.order_fall.wrapping_sub(1);
            if self.order_fall == 0 {
                maxSuccessor = minSuccessor;
                self.text = self
                    .text
                    .offset(-((self.max_context != self.min_context) as std::ffi::c_int as isize));
            }
        } else {
            self.found_state.as_mut().set_successor(maxSuccessor);
            minSuccessor =
                self.min_context.cast().offset_from(self.memory_ptr) as std::ffi::c_long as u32;
        }
        mc = self.min_context;
        c = self.max_context;
        self.min_context = self
            .memory_ptr
            .offset(minSuccessor as isize)
            .cast::<Context>();
        self.max_context = self.min_context;
        if c == mc {
            return;
        }
        ns = mc.as_ref().num_stats as std::ffi::c_uint;
        s0 = (mc.as_ref().union2.summ_freq as std::ffi::c_uint)
            .wrapping_sub(ns)
            .wrapping_sub(
                (self.found_state.as_ref().freq as std::ffi::c_uint)
                    .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint),
            );
        loop {
            let mut ns1: std::ffi::c_uint = 0;
            let mut sum: u32 = 0;
            ns1 = c.as_ref().num_stats as std::ffi::c_uint;
            if ns1 != 1 as std::ffi::c_int as std::ffi::c_uint {
                if ns1 & 1 as std::ffi::c_int as std::ffi::c_uint
                    == 0 as std::ffi::c_int as std::ffi::c_uint
                {
                    let oldNU: std::ffi::c_uint = ns1 >> 1 as std::ffi::c_int;
                    let i: std::ffi::c_uint = self.units2index
                        [(oldNU as usize).wrapping_sub(1 as std::ffi::c_int as usize)]
                        as std::ffi::c_uint;
                    if i != self.units2index[(oldNU as usize)
                        .wrapping_add(1 as std::ffi::c_int as usize)
                        .wrapping_sub(1 as std::ffi::c_int as usize)]
                        as std::ffi::c_uint
                    {
                        let Some(ptr) = self
                            .alloc_units(i.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint))
                        else {
                            self.restart_model();
                            return;
                        };

                        let mut oldPtr = self
                            .memory_ptr
                            .offset(c.as_ref().union4.stats as isize)
                            .cast::<State>()
                            .cast::<u8>();
                        let mut d = ptr.cast::<u32>();
                        let mut z = oldPtr.cast::<u32>();
                        let mut n: std::ffi::c_uint = oldNU;
                        loop {
                            *d.offset(0 as std::ffi::c_int as isize).as_mut() =
                                *z.offset(0 as std::ffi::c_int as isize).as_ref();
                            *d.offset(1 as std::ffi::c_int as isize).as_mut() =
                                *z.offset(1 as std::ffi::c_int as isize).as_ref();
                            *d.offset(2 as std::ffi::c_int as isize).as_mut() =
                                *z.offset(2 as std::ffi::c_int as isize).as_ref();
                            z = z.offset(3 as std::ffi::c_int as isize);
                            d = d.offset(3 as std::ffi::c_int as isize);
                            n = n.wrapping_sub(1);
                            if !(n != 0) {
                                break;
                            }
                        }
                        self.insert_node(oldPtr, i);
                        c.as_mut().union4.stats =
                            ptr.offset_from(self.memory_ptr) as std::ffi::c_long as u32;
                    }
                }
                sum = c.as_ref().union2.summ_freq as u32;
                sum = sum.wrapping_add(
                    (((2 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(ns1) < ns)
                        as std::ffi::c_int as std::ffi::c_uint)
                        .wrapping_add((2 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(
                            ((4 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(ns1) <= ns)
                                as std::ffi::c_int as std::ffi::c_uint
                                & (sum
                                    <= (8 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(ns1))
                                    as std::ffi::c_int
                                    as std::ffi::c_uint,
                        )),
                );
            } else {
                let Some(s_1) = self.alloc_units(0 as std::ffi::c_int as std::ffi::c_uint) else {
                    self.restart_model();
                    return;
                };
                let mut s_1 = s_1.cast::<State>();

                let mut freq: std::ffi::c_uint = c.as_ref().union2.state2.freq as std::ffi::c_uint;
                s_1.as_mut().symbol = c.as_ref().union2.state2.symbol;
                s_1.as_mut().successor_0 = c.as_ref().union4.state4.successor_0;
                s_1.as_mut().successor_1 = c.as_ref().union4.state4.successor_1;
                c.as_mut().union4.stats =
                    s_1.cast().offset_from(self.memory_ptr) as std::ffi::c_long as u32;
                if freq
                    < (124 as std::ffi::c_int / 4 as std::ffi::c_int - 1 as std::ffi::c_int)
                        as std::ffi::c_uint
                {
                    freq <<= 1 as std::ffi::c_int;
                } else {
                    freq = (124 as std::ffi::c_int - 4 as std::ffi::c_int) as std::ffi::c_uint;
                }
                s_1.as_mut().freq = freq as u8;
                sum = freq.wrapping_add(self.init_esc).wrapping_add(
                    (ns > 3 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int
                        as std::ffi::c_uint,
                );
            }
            let mut s_2 = self
                .memory_ptr
                .offset(c.as_ref().union4.stats as isize)
                .cast::<State>()
                .offset(ns1 as isize);
            let mut cf: u32 = 2 as std::ffi::c_int as u32
                * sum.wrapping_add(6 as std::ffi::c_int as u32)
                * self.found_state.as_ref().freq as u32;
            let sf: u32 = s0.wrapping_add(sum);
            s_2.as_mut().symbol = self.found_state.as_ref().symbol;
            c.as_mut().num_stats =
                ns1.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint) as u16;
            s_2.as_mut().set_successor(maxSuccessor);
            if cf < 6 as std::ffi::c_int as u32 * sf {
                cf = (1 as std::ffi::c_int as u32)
                    .wrapping_add((cf > sf) as std::ffi::c_int as u32)
                    .wrapping_add(
                        (cf >= 4 as std::ffi::c_int as u32 * sf) as std::ffi::c_int as u32,
                    );
                sum = sum.wrapping_add(3 as std::ffi::c_int as u32);
            } else {
                cf = (4 as std::ffi::c_int as u32)
                    .wrapping_add(
                        (cf >= 9 as std::ffi::c_int as u32 * sf) as std::ffi::c_int as u32,
                    )
                    .wrapping_add(
                        (cf >= 12 as std::ffi::c_int as u32 * sf) as std::ffi::c_int as u32,
                    )
                    .wrapping_add(
                        (cf >= 15 as std::ffi::c_int as u32 * sf) as std::ffi::c_int as u32,
                    );
                sum = sum.wrapping_add(cf);
            }
            c.as_mut().union2.summ_freq = sum as u16;
            s_2.as_mut().freq = cf as u8;
            c = self
                .memory_ptr
                .offset(c.as_ref().suffix as isize)
                .cast::<Context>();
            if !(c != mc) {
                break;
            }
        }
    }

    #[inline(never)]
    unsafe fn rescale(&mut self) {
        let mut i: std::ffi::c_uint = 0;
        let mut adder: std::ffi::c_uint = 0;
        let mut sumFreq: std::ffi::c_uint = 0;
        let mut escFreq: std::ffi::c_uint = 0;
        let stats = self
            .memory_ptr
            .offset(self.min_context.as_ref().union4.stats as isize)
            .cast::<State>();
        let mut s = self.found_state;
        if s != stats {
            let tmp: State = *s.as_ref();
            loop {
                *s.offset(0 as std::ffi::c_int as isize).as_mut() =
                    *s.offset(-(1 as std::ffi::c_int) as isize).as_ref();
                s = s.offset(-1);
                if !(s != stats) {
                    break;
                }
            }
            *s.as_mut() = tmp;
        }
        sumFreq = s.as_ref().freq as std::ffi::c_uint;
        escFreq =
            (self.min_context.as_ref().union2.summ_freq as std::ffi::c_uint).wrapping_sub(sumFreq);
        adder = (self.order_fall != 0 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int
            as std::ffi::c_uint;
        sumFreq = sumFreq
            .wrapping_add(4 as std::ffi::c_int as std::ffi::c_uint)
            .wrapping_add(adder)
            >> 1 as std::ffi::c_int;
        i = (self.min_context.as_ref().num_stats as std::ffi::c_uint)
            .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint);
        s.as_mut().freq = sumFreq as u8;
        loop {
            s = s.offset(1);
            let mut freq: std::ffi::c_uint = s.as_ref().freq as std::ffi::c_uint;
            escFreq = escFreq.wrapping_sub(freq);
            freq = freq.wrapping_add(adder) >> 1 as std::ffi::c_int;
            sumFreq = sumFreq.wrapping_add(freq);
            s.as_mut().freq = freq as u8;
            if freq > s.offset(-(1 as std::ffi::c_int) as isize).as_ref().freq as std::ffi::c_uint {
                let tmp_0: State = *s.as_ref();
                let mut s1 = s;
                loop {
                    *s1.offset(0 as std::ffi::c_int as isize).as_mut() =
                        *s1.offset(-(1 as std::ffi::c_int) as isize).as_ref();
                    s1 = s1.offset(-1);
                    if !(s1 != stats
                        && freq
                            > s1.offset(-(1 as std::ffi::c_int) as isize).as_ref().freq
                                as std::ffi::c_uint)
                    {
                        break;
                    }
                }
                *s1.as_mut() = tmp_0;
            }
            i = i.wrapping_sub(1);
            if !(i != 0) {
                break;
            }
        }
        if s.as_ref().freq as std::ffi::c_int == 0 as std::ffi::c_int {
            let mut numStats: std::ffi::c_uint = 0;
            let mut numStatsNew: std::ffi::c_uint = 0;
            let mut n0: std::ffi::c_uint = 0;
            let mut n1: std::ffi::c_uint = 0;
            i = 0 as std::ffi::c_int as std::ffi::c_uint;
            loop {
                i = i.wrapping_add(1);
                i;
                s = s.offset(-1);
                if !(s.as_ref().freq as std::ffi::c_int == 0 as std::ffi::c_int) {
                    break;
                }
            }
            escFreq = escFreq.wrapping_add(i);
            let mut mc = self.min_context;
            numStats = mc.as_ref().num_stats as std::ffi::c_uint;
            numStatsNew = numStats.wrapping_sub(i);
            mc.as_mut().num_stats = numStatsNew as u16;
            n0 = numStats.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint)
                >> 1 as std::ffi::c_int;
            if numStatsNew == 1 as std::ffi::c_int as std::ffi::c_uint {
                let mut freq_0: std::ffi::c_uint = stats.as_ref().freq as std::ffi::c_uint;
                loop {
                    escFreq >>= 1 as std::ffi::c_int;
                    freq_0 = freq_0.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint)
                        >> 1 as std::ffi::c_int;
                    if !(escFreq > 1 as std::ffi::c_int as std::ffi::c_uint) {
                        break;
                    }
                }
                s = NonNull::new_unchecked(addr_of_mut!(mc.as_mut().union2)).cast();
                *s.as_mut() = *stats.as_ref();
                s.as_mut().freq = freq_0 as u8;
                self.found_state = s;
                self.insert_node(
                    stats.cast(),
                    self.units2index[(n0 as usize).wrapping_sub(1 as std::ffi::c_int as usize)]
                        as std::ffi::c_uint,
                );
                return;
            }
            n1 = numStatsNew.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint)
                >> 1 as std::ffi::c_int;
            if n0 != n1 {
                let i0: std::ffi::c_uint = self.units2index
                    [(n0 as usize).wrapping_sub(1 as std::ffi::c_int as usize)]
                    as std::ffi::c_uint;
                let i1: std::ffi::c_uint = self.units2index
                    [(n1 as usize).wrapping_sub(1 as std::ffi::c_int as usize)]
                    as std::ffi::c_uint;
                if i0 != i1 {
                    if self.free_list[i1 as usize] != 0 as std::ffi::c_int as u32 {
                        let ptr: NonNull<u8> = self.remove_node(i1);
                        self.min_context.as_mut().union4.stats =
                            ptr.offset_from(self.memory_ptr) as std::ffi::c_long as u32;
                        let mut d = ptr.cast::<u32>();
                        let mut z = stats.cast::<u32>();
                        let mut n: std::ffi::c_uint = n1;
                        loop {
                            *d.offset(0 as std::ffi::c_int as isize).as_mut() =
                                *z.offset(0 as std::ffi::c_int as isize).as_ref();
                            *d.offset(1 as std::ffi::c_int as isize).as_mut() =
                                *z.offset(1 as std::ffi::c_int as isize).as_ref();
                            *d.offset(2 as std::ffi::c_int as isize).as_mut() =
                                *z.offset(2 as std::ffi::c_int as isize).as_ref();
                            z = z.offset(3 as std::ffi::c_int as isize);
                            d = d.offset(3 as std::ffi::c_int as isize);
                            n = n.wrapping_sub(1);
                            if !(n != 0) {
                                break;
                            }
                        }
                        self.insert_node(stats.cast(), i0);
                    } else {
                        self.split_block(stats.cast(), i0, i1);
                    }
                }
            }
        }
        let mut mc_0 = self.min_context;
        mc_0.as_mut().union2.summ_freq = sumFreq
            .wrapping_add(escFreq)
            .wrapping_sub(escFreq >> 1 as std::ffi::c_int)
            as u16;
        self.found_state = self
            .memory_ptr
            .offset(mc_0.as_ref().union4.stats as isize)
            .cast::<State>();
    }

    pub unsafe fn make_esc_freq(
        &mut self,
        numMasked: std::ffi::c_uint,
        escFreq: &mut u32,
    ) -> *mut See {
        let mut see;
        let mut mc = self.min_context;
        let numStats: std::ffi::c_uint = mc.as_ref().num_stats as std::ffi::c_uint;
        if numStats != 256 as std::ffi::c_int as std::ffi::c_uint {
            let nonMasked: std::ffi::c_uint = numStats.wrapping_sub(numMasked);
            see =
                self.see[self.ns2index
                    [(nonMasked as usize).wrapping_sub(1 as std::ffi::c_int as usize)]
                    as std::ffi::c_uint as usize]
                    .as_mut_ptr()
                    .offset(
                        (nonMasked
                            < (self
                                .memory_ptr
                                .offset(mc.as_ref().suffix as isize)
                                .cast::<Context>()
                                .as_ref()
                                .num_stats as std::ffi::c_uint)
                                .wrapping_sub(numStats)) as std::ffi::c_int
                            as isize,
                    )
                    .offset((2 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(
                        ((mc.as_ref().union2.summ_freq as std::ffi::c_uint)
                            < (11 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(numStats))
                            as std::ffi::c_int as std::ffi::c_uint,
                    ) as isize)
                    .offset((4 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(
                        (numMasked > nonMasked) as std::ffi::c_int as std::ffi::c_uint,
                    ) as isize)
                    .offset(self.hi_bits_flag as isize);
            let summ: std::ffi::c_uint = (*see).summ as std::ffi::c_uint;
            let r: std::ffi::c_uint = summ >> (*see).shift as std::ffi::c_int;
            (*see).summ = summ.wrapping_sub(r) as u16;
            *escFreq = r.wrapping_add(
                (r == 0 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int
                    as std::ffi::c_uint,
            );
        } else {
            see = &mut self.dummy_see;
            *escFreq = 1 as std::ffi::c_int as u32;
        }

        see
    }

    unsafe fn next_context(&mut self) {
        let c = self
            .memory_ptr
            .offset(
                (self.found_state.as_ref().successor_0 as u32
                    | (self.found_state.as_ref().successor_1 as u32) << 16 as std::ffi::c_int)
                    as isize,
            )
            .cast::<Context>();
        if self.order_fall == 0 as std::ffi::c_int as std::ffi::c_uint && c.cast() > self.text {
            self.min_context = c;
            self.max_context = self.min_context;
        } else {
            self.update_model();
        };
    }

    pub unsafe fn update1(&mut self) {
        let mut s = self.found_state;
        let mut freq: std::ffi::c_uint = s.as_ref().freq as std::ffi::c_uint;
        freq = freq.wrapping_add(4 as std::ffi::c_int as std::ffi::c_uint);
        self.min_context.as_mut().union2.summ_freq = (self.min_context.as_ref().union2.summ_freq
            as std::ffi::c_int
            + 4 as std::ffi::c_int) as u16;
        s.as_mut().freq = freq as u8;
        if freq > s.offset(-1).as_ref().freq as std::ffi::c_uint {
            let tmp: State = *s.offset(0 as std::ffi::c_int as isize).as_ref();
            *s.offset(0 as std::ffi::c_int as isize).as_mut() =
                *s.offset(-(1 as std::ffi::c_int) as isize).as_ref();
            *s.offset(-1).as_mut() = tmp;
            s = s.offset(-1);
            self.found_state = s;
            if freq > 124 as std::ffi::c_int as std::ffi::c_uint {
                self.rescale();
            }
        }
        self.next_context();
    }

    pub unsafe fn update1_0(&mut self) {
        let mut s = self.found_state;
        let mut mc = self.min_context;
        let mut freq: std::ffi::c_uint = s.as_ref().freq as std::ffi::c_uint;
        let summFreq: std::ffi::c_uint = mc.as_ref().union2.summ_freq as std::ffi::c_uint;
        self.prev_success = ((2 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(freq)
            > summFreq) as std::ffi::c_int as std::ffi::c_uint;
        self.run_length += self.prev_success as i32;
        mc.as_mut().union2.summ_freq =
            summFreq.wrapping_add(4 as std::ffi::c_int as std::ffi::c_uint) as u16;
        freq = freq.wrapping_add(4 as std::ffi::c_int as std::ffi::c_uint);
        s.as_mut().freq = freq as u8;
        if freq > 124 as std::ffi::c_int as std::ffi::c_uint {
            self.rescale();
        }
        self.next_context();
    }

    pub unsafe fn update2(&mut self) {
        let mut s = self.found_state;
        let mut freq: std::ffi::c_uint = s.as_ref().freq as std::ffi::c_uint;
        freq = freq.wrapping_add(4 as std::ffi::c_int as std::ffi::c_uint);
        self.run_length = self.init_rl;
        self.min_context.as_mut().union2.summ_freq = (self.min_context.as_ref().union2.summ_freq
            as std::ffi::c_int
            + 4 as std::ffi::c_int) as u16;
        s.as_mut().freq = freq as u8;
        if freq > 124 as std::ffi::c_int as std::ffi::c_uint {
            self.rescale();
        }
        self.update_model();
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

    pub(crate) fn into_inner(self) -> R {
        let manual_drop_self = ManuallyDrop::new(self);
        unsafe {
            dealloc(
                manual_drop_self.memory_ptr.as_ptr(),
                manual_drop_self.memory_layout,
            );
        }
        let rc = unsafe { std::ptr::read(&manual_drop_self.rc) };
        let RangeDecoder { reader, .. } = rc;
        reader
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

    pub(crate) fn into_inner(self) -> W {
        let manual_drop_self = ManuallyDrop::new(self);
        unsafe {
            dealloc(
                manual_drop_self.memory_ptr.as_ptr(),
                manual_drop_self.memory_layout,
            );
        }
        let rc = unsafe { std::ptr::read(&manual_drop_self.rc) };
        let RangeEncoder { writer, .. } = rc;
        writer
    }

    pub(crate) fn flush_range_encoder(&mut self) -> Result<(), std::io::Error> {
        self.rc.flush()
    }
}

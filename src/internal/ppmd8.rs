mod decoder;
mod encoder;
mod range_coding;

use std::{
    alloc::{alloc_zeroed, dealloc, Layout},
    io::{Read, Write},
    mem::{swap, ManuallyDrop},
    ptr::{addr_of_mut, NonNull},
};

pub(crate) use range_coding::{RangeDecoder, RangeEncoder};

use super::*;
use crate::{Error, RestoreMethod};

const MAX_FREQ: u8 = 124;
const UNIT_SIZE: isize = 12;
const K_TOP_VALUE: u32 = 1 << 24;
const K_BOT_VALUE: u32 = 1 << 15;
const EMPTY_NODE: u32 = u32::MAX;
const FLAG_RESCALED: u8 = 1 << 2;
const FLAG_PREV_HIGH: u8 = 1 << 4;

static K_EXP_ESCAPE: [u8; 16] = [25, 14, 9, 7, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 2, 2];

static K_INIT_BIN_ESC: [u16; 8] = [
    0x3CDD, 0x1F3F, 0x59BF, 0x48F3, 0x64A1, 0x5ABC, 0x6632, 0x6051,
];

#[derive(Copy, Clone)]
#[repr(C)]
struct Node {
    stamp: u32,
    next: u32,
    nu: u32,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct Context {
    num_stats: u8,
    flags: u8,
    union2: Union2,
    union4: Union4,
    suffix: u32,
}

pub(crate) struct PPMd8<RC> {
    min_context: NonNull<Context>,
    max_context: NonNull<Context>,
    found_state: NonNull<State>,
    order_fall: std::ffi::c_uint,
    init_esc: std::ffi::c_uint,
    prev_success: std::ffi::c_uint,
    max_order: std::ffi::c_uint,
    restore_method: RestoreMethod,
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
    stamps: [u32; 38],
    ns2bs_index: [u8; 256],
    ns2index: [u8; 260],
    exp_escape: [u8; 16],
    dummy_see: See,
    see: [[See; 32]; 24],
    bin_summ: [[u16; 64]; 25],
    memory_ptr: NonNull<u8>,
    memory_layout: Layout,
    rc: RC,
}

impl<RC> Drop for PPMd8<RC> {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.memory_ptr.as_ptr(), self.memory_layout);
        }
    }
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
            max_order,
            restore_method,
            run_length: 0,
            init_rl: 0,
            size: mem_size,
            glue_count: 0,
            align_offset,
            lo_unit: NonNull::dangling(),
            hi_unit: NonNull::dangling(),
            text: NonNull::dangling(),
            units_start: NonNull::dangling(),
            index2units,
            units2index,
            free_list: [0; 38],
            stamps: [0; 38],
            ns2bs_index,
            ns2index,
            exp_escape: K_EXP_ESCAPE,
            dummy_see: See::default(),
            see: [[See::default(); 32]; 24],
            bin_summ: [[0; 64]; 25],
            memory_ptr,
            memory_layout,
            rc,
        };

        unsafe { ppmd.restart_model() };

        Ok(ppmd)
    }

    unsafe fn insert_node(&mut self, mut node: NonNull<Node>, index: std::ffi::c_uint) {
        unsafe {
            node.as_mut().stamp = 0xFFFFFFFF as std::ffi::c_uint;
            node.as_mut().next = self.free_list[index as usize];
            node.as_mut().nu = self.index2units[index as usize] as std::ffi::c_uint;
            self.free_list[index as usize] =
                u32::try_from(node.cast().offset_from(self.memory_ptr))
                    .expect("Failed to convert ptr to offset");
            self.stamps[index as usize] = self.stamps[index as usize].wrapping_add(1);
        }
    }

    unsafe fn remove_node(&mut self, index: u32) -> NonNull<u8> {
        let index = index as usize;
        let node = self
            .memory_ptr
            .offset(self.free_list[index] as isize)
            .cast::<Node>();
        self.free_list[index] = node.as_ref().next;
        self.stamps[index] = self.stamps[index].wrapping_sub(1);
        self.stamps[index];
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
                ptr.offset((k * 12 as std::ffi::c_int as u32) as isize)
                    .cast(),
                nu.wrapping_sub(k)
                    .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint),
            );
        }
        self.insert_node(ptr.cast(), i);
    }

    unsafe fn glue_free_blocks(&mut self) {
        let mut n: u32 = 0;
        self.glue_count = ((1 as std::ffi::c_int) << 13 as std::ffi::c_int) as u32;
        self.stamps = [0; 38];

        if self.lo_unit != self.hi_unit {
            self.lo_unit.cast::<Node>().as_mut().stamp = 0
        }
        let mut prev: *mut u32 = &mut n;
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
            let mut next: u32 = self.free_list[i as usize];
            self.free_list[i as usize] = 0 as std::ffi::c_int as u32;
            while next != 0 as std::ffi::c_int as u32 {
                let mut node = self.memory_ptr.offset(next as isize).cast::<Node>();
                let mut nu = node.as_ref().nu;
                *prev = next;
                next = node.as_ref().next;
                if nu != 0 as std::ffi::c_int as u32 {
                    prev = &mut node.as_mut().next;
                    loop {
                        let mut node2 = node.offset(nu as isize);
                        if !(node2.as_mut().stamp == 0xFFFFFFFF as std::ffi::c_uint) {
                            break;
                        }
                        nu = nu.wrapping_add(node2.as_ref().nu);
                        node2.as_mut().nu = 0 as std::ffi::c_int as u32;
                        node.as_mut().nu = nu;
                    }
                }
            }
            i = i.wrapping_add(1);
        }
        *prev = 0 as std::ffi::c_int as u32;
        while n != 0 as std::ffi::c_int as u32 {
            let mut node_0 = self.memory_ptr.offset(n as isize).cast::<Node>();
            let mut nu_0: u32 = node_0.as_ref().nu;
            let mut i_0: std::ffi::c_uint = 0;
            n = node_0.as_ref().next;
            if nu_0 == 0 as std::ffi::c_int as u32 {
                continue;
            }
            while nu_0 > 128 as std::ffi::c_int as u32 {
                self.insert_node(
                    node_0,
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
                nu_0 = nu_0.wrapping_sub(128 as std::ffi::c_int as u32);
                node_0 = node_0.offset(128 as std::ffi::c_int as isize);
            }
            i_0 = self.units2index
                [(nu_0 as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
                as std::ffi::c_uint;
            if self.index2units[i_0 as usize] as std::ffi::c_uint != nu_0 {
                i_0 = i_0.wrapping_sub(1);
                let k: std::ffi::c_uint = self.index2units[i_0 as usize] as std::ffi::c_uint;
                self.insert_node(
                    node_0.offset(k as isize),
                    nu_0.wrapping_sub(k)
                        .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint),
                );
            }
            self.insert_node(node_0, i_0);
        }
    }

    #[inline(never)]
    unsafe fn alloc_units_rare(&mut self, index: std::ffi::c_uint) -> Option<NonNull<u8>> {
        let mut i: std::ffi::c_uint = 0;
        if self.glue_count == 0 {
            self.glue_free_blocks();
            if self.free_list[index as usize] != 0 as std::ffi::c_int as u32 {
                return Some(self.remove_node(index));
            }
        }
        i = index;
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
                let numBytes: u32 = self.index2units[index as usize] as std::ffi::c_uint
                    * 12 as std::ffi::c_int as u32;
                let us = self.units_start;
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
        self.split_block(block, i, index);
        Some(block.cast())
    }

    unsafe fn alloc_units(&mut self, indx: std::ffi::c_uint) -> Option<NonNull<u8>> {
        if self.free_list[indx as usize] != 0 {
            return Some(self.remove_node(indx));
        }
        let numBytes: u32 =
            self.index2units[indx as usize] as std::ffi::c_uint * 12 as std::ffi::c_int as u32;
        let lo = self.lo_unit;
        if self.hi_unit.offset_from(lo) as u32 >= numBytes {
            self.lo_unit = lo.offset(numBytes as isize);
            return Some(lo);
        }
        self.alloc_units_rare(indx)
    }

    unsafe fn shrink_units(
        &mut self,
        oldPtr: NonNull<u8>,
        oldNU: std::ffi::c_uint,
        newNU: std::ffi::c_uint,
    ) -> NonNull<u8> {
        let i0: std::ffi::c_uint = self.units2index
            [(oldNU as usize).wrapping_sub(1 as std::ffi::c_int as usize)]
            as std::ffi::c_uint;
        let i1: std::ffi::c_uint = self.units2index
            [(newNU as usize).wrapping_sub(1 as std::ffi::c_int as usize)]
            as std::ffi::c_uint;
        if i0 == i1 {
            return oldPtr;
        }
        if self.free_list[i1 as usize] != 0 as std::ffi::c_int as u32 {
            let ptr = self.remove_node(i1);
            let mut d = ptr.cast::<u32>();
            let mut z = oldPtr.cast::<u32>();
            let mut n: std::ffi::c_uint = newNU;
            loop {
                *d.offset(0).as_mut() = *z.offset(0).as_ref();
                *d.offset(1).as_mut() = *z.offset(1).as_ref();
                *d.offset(2).as_mut() = *z.offset(2).as_ref();
                z = z.offset(3);
                d = d.offset(3);
                n = n.wrapping_sub(1);
                if !(n != 0) {
                    break;
                }
            }
            self.insert_node(oldPtr.cast(), i0);
            return ptr;
        }
        self.split_block(oldPtr, i0, i1);
        oldPtr
    }

    unsafe fn free_units(&mut self, ptr: NonNull<u8>, nu: std::ffi::c_uint) {
        self.insert_node(
            ptr.cast(),
            self.units2index[(nu as usize).wrapping_sub(1 as std::ffi::c_int as usize)]
                as std::ffi::c_uint,
        );
    }

    unsafe fn special_free_unit(&mut self, ptr: NonNull<u8>) {
        if ptr != self.units_start {
            self.insert_node(ptr.cast(), 0);
        } else {
            self.units_start = self.units_start.offset(12 as std::ffi::c_int as isize);
        };
    }

    unsafe fn expand_text_area(&mut self) {
        let mut count: [u32; 38] = [0; 38];
        let mut i: std::ffi::c_uint = 0;
        if self.lo_unit != self.hi_unit {
            self.lo_unit.cast::<Node>().as_mut().stamp = 0 as std::ffi::c_int as u32;
        }
        let mut node = self.units_start.cast::<Node>();
        while node.as_ref().stamp == 0xFFFFFFFF {
            let nu: u32 = node.as_ref().nu;
            node.as_mut().stamp = 0;
            count[self.units2index[(nu as usize).wrapping_sub(1 as std::ffi::c_int as usize)]
                as usize] = (count[self.units2index
                [(nu as usize).wrapping_sub(1 as std::ffi::c_int as usize)]
                as usize])
                .wrapping_add(1);
            count[self.units2index[(nu as usize).wrapping_sub(1 as std::ffi::c_int as usize)]
                as usize];
            node = node.offset(nu as isize);
        }
        self.units_start = node.cast();
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
            let mut cnt: u32 = count[i as usize];
            if !(cnt == 0 as std::ffi::c_int as u32) {
                let mut prev: *mut u32 =
                    &mut *self.free_list.as_mut_ptr().offset(i as isize) as *mut u32;
                let mut n: u32 = *prev;
                self.stamps[i as usize] = (self.stamps[i as usize]).wrapping_sub(cnt);
                loop {
                    let mut node_0 = self.memory_ptr.offset(n as isize).cast::<Node>();
                    n = node_0.as_ref().next;
                    if node_0.as_ref().stamp != 0 as std::ffi::c_int as u32 {
                        prev = &mut node_0.as_mut().next;
                    } else {
                        *prev = n;
                        cnt = cnt.wrapping_sub(1);
                        if cnt == 0 as std::ffi::c_int as u32 {
                            break;
                        }
                    }
                }
            }
            i = i.wrapping_add(1);
        }
    }

    #[inline(never)]
    unsafe fn restart_model(&mut self) {
        let mut i: std::ffi::c_uint = 0;
        let mut k: std::ffi::c_uint = 0;
        let mut m: std::ffi::c_uint = 0;
        self.free_list = [0; 38];
        self.stamps = [0; 38];
        self.text = self
            .memory_ptr
            .offset(self.align_offset as isize)
            .offset(0 as std::ffi::c_int as isize);
        self.hi_unit = self.text.offset(self.size as isize);
        self.units_start = self.hi_unit.offset(
            -((self.size / 8 as std::ffi::c_int as u32 / 12 as std::ffi::c_int as u32
                * 7 as std::ffi::c_int as u32
                * 12 as std::ffi::c_int as u32) as isize),
        );
        self.lo_unit = self.units_start;
        self.glue_count = 0 as std::ffi::c_int as u32;
        self.order_fall = self.max_order;
        self.init_rl = -((if self.max_order < 12 {
            self.max_order
        } else {
            12
        }) as i32)
            - 1;
        self.run_length = self.init_rl;
        self.prev_success = 0 as std::ffi::c_int as std::ffi::c_uint;
        self.hi_unit = self.hi_unit.offset(-(12 as std::ffi::c_int as isize));
        let mut mc = self.hi_unit.cast::<Context>();
        let mut s = self.lo_unit.cast::<State>();
        self.lo_unit = self
            .lo_unit
            .offset(((256 / 2) as u32 * 12 as std::ffi::c_int as u32) as isize);
        self.min_context = mc;
        self.max_context = self.min_context;
        self.found_state = s;
        mc.as_mut().flags = 0;
        mc.as_mut().num_stats = (256 - 1) as u8;
        mc.as_mut().union2.summ_freq = (256 as std::ffi::c_int + 1 as std::ffi::c_int) as u16;
        mc.as_mut().union4.stats = s.cast().offset_from(self.memory_ptr) as u32;
        mc.as_mut().suffix = 0;
        i = 0;
        while i < 256 as std::ffi::c_int as std::ffi::c_uint {
            s.as_mut().symbol = i as u8;
            s.as_mut().freq = 1 as std::ffi::c_int as u8;
            s.as_mut().set_successor(0);
            i = i.wrapping_add(1);
            s = s.offset(1);
        }
        m = 0;
        i = m;
        while m < 25 as std::ffi::c_int as std::ffi::c_uint {
            while self.ns2index[i as usize] as std::ffi::c_uint == m {
                i = i.wrapping_add(1);
                i;
            }
            k = 0 as std::ffi::c_int as std::ffi::c_uint;
            while k < 8 as std::ffi::c_int as std::ffi::c_uint {
                let mut r: std::ffi::c_uint = 0;
                let dest: *mut u16 = self.bin_summ[m as usize].as_mut_ptr().offset(k as isize);
                let val: u16 = (((1 as std::ffi::c_int)
                    << 7 as std::ffi::c_int + 7 as std::ffi::c_int)
                    as std::ffi::c_uint)
                    .wrapping_sub(
                        (K_INIT_BIN_ESC[k as usize] as std::ffi::c_uint)
                            .wrapping_div(i.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint)),
                    ) as u16;
                r = 0 as std::ffi::c_int as std::ffi::c_uint;
                while r < 64 as std::ffi::c_int as std::ffi::c_uint {
                    *dest.offset(r as isize) = val;
                    r = r.wrapping_add(8 as std::ffi::c_int as std::ffi::c_uint);
                }
                k = k.wrapping_add(1);
            }
            m = m.wrapping_add(1);
        }
        m = 0 as std::ffi::c_int as std::ffi::c_uint;
        i = m;
        while m < 24 as std::ffi::c_int as std::ffi::c_uint {
            let mut summ: std::ffi::c_uint = 0;
            while self.ns2index[(i as usize).wrapping_add(3 as std::ffi::c_int as usize) as usize]
                as std::ffi::c_uint
                == m.wrapping_add(3 as std::ffi::c_int as std::ffi::c_uint)
            {
                i = i.wrapping_add(1);
            }
            let mut s_0 = self.see[m as usize].as_mut_ptr();
            summ = (2 as std::ffi::c_int as std::ffi::c_uint)
                .wrapping_mul(i)
                .wrapping_add(5 as std::ffi::c_int as std::ffi::c_uint)
                << 7 as std::ffi::c_int - 4 as std::ffi::c_int;
            k = 0 as std::ffi::c_int as std::ffi::c_uint;
            while k < 32 as std::ffi::c_int as std::ffi::c_uint {
                (*s_0).summ = summ as u16;
                (*s_0).shift = (7 as std::ffi::c_int - 4 as std::ffi::c_int) as u8;
                (*s_0).count = 7 as std::ffi::c_int as u8;
                k = k.wrapping_add(1);
                s_0 = s_0.offset(1);
            }
            m = m.wrapping_add(1);
        }
        self.dummy_see.summ = 0 as std::ffi::c_int as u16;
        self.dummy_see.shift = 7 as std::ffi::c_int as u8;
        self.dummy_see.count = 64 as std::ffi::c_int as u8;
    }

    unsafe fn refresh(
        &mut self,
        mut ctx: NonNull<Context>,
        oldNU: std::ffi::c_uint,
        mut scale: std::ffi::c_uint,
    ) {
        let mut i: std::ffi::c_uint = ctx.as_ref().num_stats as std::ffi::c_uint;
        let mut escFreq: std::ffi::c_uint = 0;
        let mut sumFreq: std::ffi::c_uint = 0;
        let mut flags: std::ffi::c_uint = 0;
        let mut s = self
            .shrink_units(
                self.memory_ptr.offset(ctx.as_ref().union4.stats as isize),
                oldNU,
                i.wrapping_add(2 as std::ffi::c_int as std::ffi::c_uint) >> 1 as std::ffi::c_int,
            )
            .cast::<State>();
        ctx.as_mut().union4.stats = s.cast().offset_from(self.memory_ptr) as u32;
        scale |= (ctx.as_ref().union2.summ_freq as u32
            >= (1 as std::ffi::c_int as u32) << 15 as std::ffi::c_int)
            as std::ffi::c_int as std::ffi::c_uint;
        flags = (s.as_ref().symbol as std::ffi::c_uint)
            .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint);
        let mut freq: std::ffi::c_uint = s.as_ref().freq as std::ffi::c_uint;
        escFreq = (ctx.as_ref().union2.summ_freq as std::ffi::c_uint).wrapping_sub(freq);
        freq = freq.wrapping_add(scale) >> scale;
        sumFreq = freq;
        s.as_mut().freq = freq as u8;
        loop {
            s = s.offset(1);
            let mut freq_0: std::ffi::c_uint = s.as_ref().freq as std::ffi::c_uint;
            escFreq = escFreq.wrapping_sub(freq_0);
            freq_0 = freq_0.wrapping_add(scale) >> scale;
            sumFreq = sumFreq.wrapping_add(freq_0);
            s.as_mut().freq = freq_0 as u8;
            flags |= (s.as_ref().symbol as std::ffi::c_uint)
                .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint);
            i = i.wrapping_sub(1);
            if !(i != 0) {
                break;
            }
        }
        ctx.as_mut().union2.summ_freq =
            sumFreq.wrapping_add(escFreq.wrapping_add(scale) >> scale) as u16;
        ctx.as_mut().flags = (ctx.as_ref().flags as std::ffi::c_uint
            & (((1 as std::ffi::c_int) << 4 as std::ffi::c_int) as std::ffi::c_uint).wrapping_add(
                (((1 as std::ffi::c_int) << 2 as std::ffi::c_int) as std::ffi::c_uint)
                    .wrapping_mul(scale),
            ))
        .wrapping_add(
            flags >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
                & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint,
        ) as u8;
    }

    unsafe fn cut_off(&mut self, mut ctx: NonNull<Context>, order: std::ffi::c_uint) -> u32 {
        let mut ns: std::ffi::c_int = ctx.as_ref().num_stats as std::ffi::c_int;
        let mut nu: std::ffi::c_uint = 0;

        if ns == 0 as std::ffi::c_int {
            let mut s = NonNull::new_unchecked(addr_of_mut!(ctx.as_mut().union2)).cast::<State>();
            let mut successor: u32 = s.as_ref().successor_0 as u32
                | (s.as_ref().successor_1 as u32) << 16 as std::ffi::c_int;
            if self.memory_ptr.offset(successor as isize) >= self.units_start {
                if order < self.max_order {
                    successor = self.cut_off(
                        self.memory_ptr.offset(successor as isize).cast::<Context>(),
                        order.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint),
                    );
                } else {
                    successor = 0 as std::ffi::c_int as u32;
                }
                s.as_mut().set_successor(successor);
                if successor != 0 || order <= 9 as std::ffi::c_int as std::ffi::c_uint {
                    return ctx.cast().offset_from(self.memory_ptr) as u32;
                }
            }
            self.special_free_unit(ctx.cast());
            return 0 as std::ffi::c_int as u32;
        }
        nu = (ns as std::ffi::c_uint).wrapping_add(2 as std::ffi::c_int as std::ffi::c_uint)
            >> 1 as std::ffi::c_int;
        let indx: std::ffi::c_uint = self.units2index
            [(nu as usize).wrapping_sub(1 as std::ffi::c_int as usize)]
            as std::ffi::c_uint;
        let mut stats = self
            .memory_ptr
            .offset(ctx.as_ref().union4.stats as isize)
            .cast::<State>();
        if stats.cast().offset_from(self.units_start) as std::ffi::c_long as u32
            <= ((1 as std::ffi::c_int) << 14 as std::ffi::c_int) as u32
            && ctx.as_ref().union4.stats <= self.free_list[indx as usize]
        {
            let ptr = self.remove_node(indx);
            ctx.as_mut().union4.stats = ptr.offset_from(self.memory_ptr) as std::ffi::c_long as u32;
            let mut d = ptr.cast::<u32>();
            let mut z = stats.cast::<u32>();
            let mut n: std::ffi::c_uint = nu;
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
            if stats.cast() != self.units_start {
                self.insert_node(stats.cast(), indx);
            } else {
                self.units_start = self.units_start.offset(
                    (self.index2units[indx as usize] as std::ffi::c_uint
                        * 12 as std::ffi::c_int as u32) as isize,
                );
            }
            stats = ptr.cast();
        }
        let mut s_0 = stats.offset(ns as u32 as isize);
        loop {
            let successor_0: u32 = s_0.as_ref().successor_0 as u32
                | (s_0.as_ref().successor_1 as u32) << 16 as std::ffi::c_int;
            if self.memory_ptr.offset(successor_0 as isize) < self.units_start {
                let fresh1 = ns;
                ns = ns - 1;
                let mut s2 = stats.offset(fresh1 as std::ffi::c_uint as isize);
                if order != 0 {
                    if s_0 != s2 {
                        *s_0.as_mut() = *s2.as_ref();
                    }
                } else {
                    swap(s_0.as_mut(), s2.as_mut());
                    s2.as_mut().set_successor(0);
                }
            } else if order < self.max_order {
                s_0.as_mut().set_successor(
                    self.cut_off(
                        self.memory_ptr
                            .offset(successor_0 as isize)
                            .cast::<Context>(),
                        order.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint),
                    ),
                );
            } else {
                s_0.as_mut().set_successor(0 as std::ffi::c_int as u32);
            }
            s_0 = s_0.offset(-1);
            if !(s_0 >= stats) {
                break;
            }
        }
        if ns != ctx.as_ref().num_stats as std::ffi::c_int && order != 0 {
            if ns < 0 as std::ffi::c_int {
                self.free_units(stats.cast(), nu);
                self.special_free_unit(ctx.cast());
                return 0;
            }
            ctx.as_mut().num_stats = ns as u8;
            if ns == 0 as std::ffi::c_int {
                let sym: u8 = stats.as_ref().symbol;
                ctx.as_mut().flags = ((ctx.as_ref().flags as std::ffi::c_int
                    & (1 as std::ffi::c_int) << 4 as std::ffi::c_int)
                    as std::ffi::c_uint)
                    .wrapping_add(
                        (sym as std::ffi::c_uint)
                            .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
                            >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
                            & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint,
                    ) as u8;
                ctx.as_mut().union2.state2.symbol = sym;
                ctx.as_mut().union2.state2.freq = ((stats.as_ref().freq as std::ffi::c_uint)
                    .wrapping_add(11 as std::ffi::c_int as std::ffi::c_uint)
                    >> 3 as std::ffi::c_int)
                    as u8;
                ctx.as_mut().union4.state4.successor_0 = stats.as_ref().successor_0;
                ctx.as_mut().union4.state4.successor_1 = stats.as_ref().successor_1;
                self.free_units(stats.cast(), nu);
            } else {
                self.refresh(
                    ctx,
                    nu,
                    (ctx.as_ref().union2.summ_freq as std::ffi::c_uint
                        > (16 as std::ffi::c_int as std::ffi::c_uint)
                            .wrapping_mul(ns as std::ffi::c_uint))
                        as std::ffi::c_int as std::ffi::c_uint,
                );
            }
        }
        ctx.cast().offset_from(self.memory_ptr) as std::ffi::c_long as u32
    }

    unsafe fn get_used_memory(&self) -> u32 {
        let mut v: u32 = 0 as std::ffi::c_int as u32;
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
            v = (v as std::ffi::c_uint).wrapping_add(
                (self.stamps[i as usize])
                    .wrapping_mul(self.index2units[i as usize] as std::ffi::c_uint),
            );
            i = i.wrapping_add(1);
        }
        self.size
            .wrapping_sub((self.hi_unit).offset_from(self.lo_unit) as std::ffi::c_long as u32)
            .wrapping_sub((self.units_start).offset_from(self.text) as std::ffi::c_long as u32)
            .wrapping_sub(v * 12 as std::ffi::c_int as u32)
    }

    unsafe fn restore_model(&mut self, ctxError: NonNull<Context>) {
        self.text = self
            .memory_ptr
            .offset(self.align_offset as isize)
            .offset(0 as std::ffi::c_int as isize);
        let mut c = self.max_context;
        let mut s: NonNull<State>;
        while c != ctxError {
            c.as_mut().num_stats = c.as_ref().num_stats.wrapping_sub(1);
            if c.as_ref().num_stats as std::ffi::c_int == 0 as std::ffi::c_int {
                s = self
                    .memory_ptr
                    .offset(c.as_ref().union4.stats as isize)
                    .cast();
                c.as_mut().flags = ((c.as_ref().flags as std::ffi::c_int
                    & (1 as std::ffi::c_int) << 4 as std::ffi::c_int)
                    as std::ffi::c_uint)
                    .wrapping_add(
                        (s.as_ref().symbol as std::ffi::c_uint)
                            .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
                            >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
                            & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint,
                    ) as u8;
                c.as_mut().union2.state2.symbol = s.as_ref().symbol;
                c.as_mut().union2.state2.freq = ((s.as_ref().freq as std::ffi::c_uint)
                    .wrapping_add(11 as std::ffi::c_int as std::ffi::c_uint)
                    >> 3 as std::ffi::c_int) as u8;
                c.as_mut().union4.state4.successor_0 = s.as_ref().successor_0;
                c.as_mut().union4.state4.successor_1 = s.as_ref().successor_1;
                self.special_free_unit(s.cast());
            } else {
                self.refresh(
                    c,
                    (c.as_ref().num_stats as std::ffi::c_uint)
                        .wrapping_add(3 as std::ffi::c_int as std::ffi::c_uint)
                        >> 1 as std::ffi::c_int,
                    0 as std::ffi::c_int as std::ffi::c_uint,
                );
            }
            c = self.memory_ptr.offset(c.as_ref().suffix as isize).cast();
        }
        while c != self.min_context {
            if c.as_ref().num_stats as std::ffi::c_int == 0 as std::ffi::c_int {
                c.as_mut().union2.state2.freq = ((c.as_ref().union2.state2.freq
                    as std::ffi::c_uint)
                    .wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint)
                    >> 1 as std::ffi::c_int) as u8;
            } else {
                c.as_mut().union2.summ_freq =
                    (c.as_ref().union2.summ_freq as std::ffi::c_int + 4 as std::ffi::c_int) as u16;
                if c.as_ref().union2.summ_freq as std::ffi::c_int
                    > 128 as std::ffi::c_int
                        + 4 as std::ffi::c_int * c.as_ref().num_stats as std::ffi::c_int
                {
                    self.refresh(
                        c,
                        (c.as_ref().num_stats as std::ffi::c_uint)
                            .wrapping_add(2 as std::ffi::c_int as std::ffi::c_uint)
                            >> 1 as std::ffi::c_int,
                        1 as std::ffi::c_int as std::ffi::c_uint,
                    );
                }
            }
            c = self.memory_ptr.offset(c.as_ref().suffix as isize).cast();
        }
        if self.restore_method == RestoreMethod::Restart
            || self.get_used_memory() < self.size >> 1 as std::ffi::c_int
        {
            self.restart_model();
        } else {
            while self.max_context.as_ref().suffix != 0 {
                self.max_context = self
                    .memory_ptr
                    .offset(self.max_context.as_ref().suffix as isize)
                    .cast();
            }
            loop {
                self.cut_off(self.max_context, 0 as std::ffi::c_int as std::ffi::c_uint);
                self.expand_text_area();
                if !(self.get_used_memory()
                    > 3 as std::ffi::c_int as u32 * (self.size >> 2 as std::ffi::c_int))
                {
                    break;
                }
            }
            self.glue_count = 0 as std::ffi::c_int as u32;
            self.order_fall = self.max_order;
        }
        self.min_context = self.max_context;
    }

    #[inline(never)]
    unsafe fn create_successors(
        &mut self,
        skip: i32,
        mut s1: &mut Option<NonNull<State>>,
        mut c: NonNull<Context>,
    ) -> Option<NonNull<Context>> {
        let mut upBranch: u32 = self.found_state.as_ref().successor_0 as u32
            | (self.found_state.as_ref().successor_1 as u32) << 16 as std::ffi::c_int;
        let mut newSym: u8 = 0;
        let mut newFreq: u8 = 0;
        let mut flags: u8 = 0;
        let mut num_ps: std::ffi::c_uint = 0 as std::ffi::c_int as std::ffi::c_uint;
        let mut ps: [Option<NonNull<State>>; 17] = [None; 17];
        if skip == 0 {
            let fresh2 = num_ps;
            num_ps = num_ps.wrapping_add(1);
            ps[fresh2 as usize] = Some(self.found_state);
        }
        while c.as_ref().suffix != 0 {
            let mut successor: u32 = 0;
            let mut s: NonNull<State>;
            c = self.memory_ptr.offset(c.as_ref().suffix as isize).cast();
            if let Some(state) = s1 {
                s = *state;
                *s1 = None;
            } else if c.as_ref().num_stats as std::ffi::c_int != 0 as std::ffi::c_int {
                let sym: u8 = self.found_state.as_ref().symbol;
                s = self
                    .memory_ptr
                    .offset(c.as_ref().union4.stats as isize)
                    .cast();
                while s.as_ref().symbol as std::ffi::c_int != sym as std::ffi::c_int {
                    s = s.offset(1);
                }
                if (s.as_ref().freq as std::ffi::c_int)
                    < 124 as std::ffi::c_int - 9 as std::ffi::c_int
                {
                    s.as_mut().freq = s.as_ref().freq.wrapping_add(1);
                    c.as_mut().union2.summ_freq = (c.as_ref().union2.summ_freq).wrapping_add(1);
                }
            } else {
                s = NonNull::new_unchecked(addr_of_mut!(c.as_mut().union2)).cast();
                s.as_mut().freq = (s.as_ref().freq as std::ffi::c_int
                    + ((self
                        .memory_ptr
                        .offset(c.as_ref().suffix as isize)
                        .cast::<Context>()
                        .as_ref()
                        .num_stats
                        == 0) as std::ffi::c_int
                        & ((s.as_ref().freq as std::ffi::c_int) < 24 as std::ffi::c_int)
                            as std::ffi::c_int)) as u8;
            }
            successor = s.as_ref().successor_0 as u32
                | (s.as_ref().successor_1 as u32) << 16 as std::ffi::c_int;
            if successor != upBranch {
                c = self.memory_ptr.offset(successor as isize).cast();
                if num_ps == 0 as std::ffi::c_int as std::ffi::c_uint {
                    return Some(c);
                }
                break;
            } else {
                let fresh3 = num_ps;
                num_ps = num_ps.wrapping_add(1);
                ps[fresh3 as usize] = Some(s);
            }
        }
        newSym = *self.memory_ptr.offset(upBranch as isize).as_ref();
        upBranch = upBranch.wrapping_add(1);
        flags = ((self.found_state.as_ref().symbol as std::ffi::c_uint)
            .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
            >> 8 as std::ffi::c_int - 4 as std::ffi::c_int
            & ((1 as std::ffi::c_int) << 4 as std::ffi::c_int) as std::ffi::c_uint)
            .wrapping_add(
                (newSym as std::ffi::c_uint)
                    .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
                    >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
                    & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint,
            ) as u8;
        if c.as_ref().num_stats as std::ffi::c_int == 0 as std::ffi::c_int {
            newFreq = c.as_ref().union2.state2.freq;
        } else {
            let mut cf: u32 = 0;
            let mut s0: u32 = 0;

            let mut s_0 = self
                .memory_ptr
                .offset(c.as_ref().union4.stats as isize)
                .cast::<State>();
            while s_0.as_ref().symbol as std::ffi::c_int != newSym as std::ffi::c_int {
                s_0 = s_0.offset(1);
            }
            cf = (s_0.as_ref().freq as u32).wrapping_sub(1 as std::ffi::c_int as u32);
            s0 = (c.as_ref().union2.summ_freq as u32)
                .wrapping_sub(c.as_ref().num_stats as u32)
                .wrapping_sub(cf);
            newFreq = (1 as std::ffi::c_int as u32).wrapping_add(
                if 2 as std::ffi::c_int as u32 * cf <= s0 {
                    (5 as std::ffi::c_int as u32 * cf > s0) as std::ffi::c_int as u32
                } else {
                    cf.wrapping_add(2 as std::ffi::c_int as u32 * s0)
                        .wrapping_sub(3 as std::ffi::c_int as u32)
                        / s0
                },
            ) as u8;
        }
        loop {
            let mut c1: NonNull<Context>;
            if self.hi_unit != self.lo_unit {
                self.hi_unit = self.hi_unit.offset(-(12 as std::ffi::c_int as isize));
                c1 = self.hi_unit.cast();
            } else if self.free_list[0 as std::ffi::c_int as usize] != 0 as std::ffi::c_int as u32 {
                c1 = self.remove_node(0).cast();
            } else {
                let Some(context) = self.alloc_units_rare(0) else {
                    return None;
                };
                c1 = context.cast();
            }
            c1.as_mut().flags = flags;
            c1.as_mut().num_stats = 0 as std::ffi::c_int as u8;
            c1.as_mut().union2.state2.symbol = newSym;
            c1.as_mut().union2.state2.freq = newFreq;
            let state = addr_of_mut!(c1.as_mut().union2).cast::<State>();
            (*state).set_successor(upBranch);
            c1.as_mut().suffix = c.cast().offset_from(self.memory_ptr) as std::ffi::c_long as u32;
            num_ps = num_ps.wrapping_sub(1);
            ps[num_ps as usize]
                .as_mut()
                .expect("ps[num_ps] was not set")
                .as_mut()
                .set_successor(c1.cast().offset_from(self.memory_ptr) as std::ffi::c_long as u32);
            c = c1;
            if !(num_ps != 0 as std::ffi::c_int as std::ffi::c_uint) {
                break;
            }
        }

        Some(c)
    }

    unsafe fn reduce_order(
        &mut self,
        mut s1: &mut Option<NonNull<State>>,
        mut c: NonNull<Context>,
    ) -> Option<NonNull<Context>> {
        let mut s: NonNull<State>;
        let c1 = c;
        let upBranch: u32 = (self.text).offset_from(self.memory_ptr) as std::ffi::c_long as u32;
        self.found_state.as_mut().set_successor(upBranch);
        self.order_fall = (self.order_fall).wrapping_add(1);
        self.order_fall;
        loop {
            if let Some(state) = s1 {
                c = self.memory_ptr.offset(c.as_ref().suffix as isize).cast();
                s = *state;
                *s1 = None;
            } else {
                if c.as_ref().suffix == 0 {
                    return Some(c);
                }
                c = self.memory_ptr.offset(c.as_ref().suffix as isize).cast();
                if c.as_ref().num_stats != 0 {
                    s = self
                        .memory_ptr
                        .offset(c.as_ref().union4.stats as isize)
                        .cast();
                    if s.as_ref().symbol as std::ffi::c_int
                        != self.found_state.as_ref().symbol as std::ffi::c_int
                    {
                        loop {
                            s = s.offset(1);
                            if !(s.as_ref().symbol as std::ffi::c_int
                                != self.found_state.as_ref().symbol as std::ffi::c_int)
                            {
                                break;
                            }
                        }
                    }
                    if (s.as_ref().freq as std::ffi::c_int)
                        < 124 as std::ffi::c_int - 9 as std::ffi::c_int
                    {
                        s.as_mut().freq =
                            (s.as_ref().freq as std::ffi::c_int + 2 as std::ffi::c_int) as u8;
                        c.as_mut().union2.summ_freq =
                            (c.as_ref().union2.summ_freq as std::ffi::c_int + 2 as std::ffi::c_int)
                                as u16;
                    }
                } else {
                    s = NonNull::new_unchecked(addr_of_mut!(c.as_mut().union2)).cast();
                    s.as_mut().freq = (s.as_ref().freq as std::ffi::c_int
                        + ((s.as_ref().freq as std::ffi::c_int) < 32 as std::ffi::c_int)
                            as std::ffi::c_int) as u8;
                }
            }
            if s.as_ref().successor_0 as u32
                | (s.as_ref().successor_1 as u32) << 16 as std::ffi::c_int
                != 0
            {
                break;
            }
            s.as_mut().set_successor(upBranch);
            self.order_fall = self.order_fall.wrapping_add(1);
        }
        if s.as_ref().successor_0 as u32 | (s.as_ref().successor_1 as u32) << 16 as std::ffi::c_int
            <= upBranch
        {
            let s2 = self.found_state;
            self.found_state = s;
            match self.create_successors(0, &mut None, c) {
                None => {
                    s.as_mut().set_successor(0 as std::ffi::c_int as u32);
                }
                Some(successor) => {
                    s.as_mut()
                        .set_successor(successor.cast().offset_from(self.memory_ptr)
                            as std::ffi::c_long as u32);
                }
            }

            self.found_state = s2;
        }

        let successor_0: u32 = s.as_ref().successor_0 as u32
            | (s.as_ref().successor_1 as u32) << 16 as std::ffi::c_int;

        if self.order_fall == 1 as std::ffi::c_int as std::ffi::c_uint && c1 == self.max_context {
            self.found_state.as_mut().set_successor(successor_0);
            self.text = self.text.offset(-1);
        }

        if successor_0 == 0 {
            None
        } else {
            Some(self.memory_ptr.offset(successor_0 as isize).cast())
        }
    }

    #[inline(never)]
    pub unsafe fn update_model(&mut self) {
        let mut maxSuccessor: u32 = 0;
        let mut minSuccessor: u32 = self.found_state.as_ref().successor_0 as u32
            | (self.found_state.as_ref().successor_1 as u32) << 16 as std::ffi::c_int;
        let mut c: NonNull<Context>;
        let mut s0: std::ffi::c_uint = 0;
        let mut ns: std::ffi::c_uint = 0;
        let fFreq: std::ffi::c_uint = self.found_state.as_ref().freq as std::ffi::c_uint;
        let mut flag: u8 = 0;
        let fSymbol: u8 = self.found_state.as_ref().symbol;
        let mut s: Option<NonNull<State>> = None;

        if (self.found_state.as_ref().freq as std::ffi::c_int)
            < 124 as std::ffi::c_int / 4 as std::ffi::c_int
            && self.min_context.as_ref().suffix != 0 as std::ffi::c_int as u32
        {
            c = self
                .memory_ptr
                .offset(self.min_context.as_ref().suffix as isize)
                .cast();
            if c.as_ref().num_stats as std::ffi::c_int == 0 as std::ffi::c_int {
                let mut state: NonNull<State> =
                    NonNull::new_unchecked(addr_of_mut!(c.as_mut().union2)).cast();
                if (state.as_ref().freq as std::ffi::c_int) < 32 as std::ffi::c_int {
                    state.as_mut().freq = state.as_ref().freq.wrapping_add(1);
                    state.as_ref().freq;
                }
                s = Some(state)
            } else {
                let sym = self.found_state.as_ref().symbol;
                let mut state: NonNull<State> = self
                    .memory_ptr
                    .offset(c.as_ref().union4.stats as isize)
                    .cast();
                if state.as_ref().symbol as std::ffi::c_int != sym as std::ffi::c_int {
                    loop {
                        state = state.offset(1);
                        if !(state.as_ref().symbol as std::ffi::c_int != sym as std::ffi::c_int) {
                            break;
                        }
                    }
                    if state.offset(0 as std::ffi::c_int as isize).as_ref().freq as std::ffi::c_int
                        >= state.offset(-(1 as std::ffi::c_int) as isize).as_ref().freq
                            as std::ffi::c_int
                    {
                        swap(
                            state.offset(0 as std::ffi::c_int as isize).as_mut(),
                            state.offset(-(1 as std::ffi::c_int) as isize).as_mut(),
                        );
                        state = state.offset(-1);
                    }
                }

                if (state.as_ref().freq as std::ffi::c_int)
                    < 124 as std::ffi::c_int - 9 as std::ffi::c_int
                {
                    state.as_mut().freq =
                        (state.as_ref().freq as std::ffi::c_int + 2 as std::ffi::c_int) as u8;
                    c.as_mut().union2.summ_freq = (c.as_ref().union2.summ_freq as std::ffi::c_int
                        + 2 as std::ffi::c_int)
                        as u16;
                }

                s = Some(state);
            }
        }
        c = self.max_context;
        if self.order_fall == 0 as std::ffi::c_int as std::ffi::c_uint && minSuccessor != 0 {
            let Some(cs) = self.create_successors(1 as std::ffi::c_int, &mut s, self.min_context)
            else {
                self.found_state.as_mut().set_successor(0);
                self.restore_model(c);
                return;
            };
            self.found_state
                .as_mut()
                .set_successor(cs.cast().offset_from(self.memory_ptr) as std::ffi::c_long as u32);
            self.max_context = cs;
            self.min_context = self.max_context;
            return;
        }
        let mut text = self.text;
        let mut fresh4 = text;
        text = text.offset(1);
        *fresh4.as_mut() = self.found_state.as_ref().symbol;
        self.text = text;
        if text >= self.units_start {
            self.restore_model(c);
            return;
        }
        maxSuccessor = text.offset_from(self.memory_ptr) as std::ffi::c_long as u32;
        if minSuccessor == 0 {
            let Some(cs_0) = self.reduce_order(&mut s, self.min_context) else {
                self.restore_model(c);
                return;
            };
            minSuccessor = cs_0.cast().offset_from(self.memory_ptr) as std::ffi::c_long as u32;
        } else if self.memory_ptr.offset(minSuccessor as isize) < self.units_start {
            let Some(cs_1) = self.create_successors(0 as std::ffi::c_int, &mut s, self.min_context)
            else {
                self.restore_model(c);
                return;
            };
            minSuccessor = cs_1.cast().offset_from(self.memory_ptr) as std::ffi::c_long as u32;
        }
        self.order_fall = (self.order_fall).wrapping_sub(1);
        if self.order_fall == 0 as std::ffi::c_int as std::ffi::c_uint {
            maxSuccessor = minSuccessor;
            self.text = (self.text)
                .offset(-((self.max_context != self.min_context) as std::ffi::c_int as isize));
        }
        flag = ((fSymbol as std::ffi::c_uint)
            .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
            >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
            & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint)
            as u8;
        ns = self.min_context.as_ref().num_stats as std::ffi::c_uint;
        s0 = (self.min_context.as_ref().union2.summ_freq as std::ffi::c_uint)
            .wrapping_sub(ns)
            .wrapping_sub(fFreq);
        while c != self.min_context {
            let mut ns1: std::ffi::c_uint = 0;
            let mut sum: u32 = 0;
            ns1 = c.as_ref().num_stats as std::ffi::c_uint;
            if ns1 != 0 as std::ffi::c_int as std::ffi::c_uint {
                if ns1 & 1 as std::ffi::c_int as std::ffi::c_uint
                    != 0 as std::ffi::c_int as std::ffi::c_uint
                {
                    let oldNU: std::ffi::c_uint = ns1
                        .wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint)
                        >> 1 as std::ffi::c_int;
                    let i: std::ffi::c_uint = self.units2index
                        [(oldNU as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
                        as std::ffi::c_uint;
                    if i != self.units2index[(oldNU as usize)
                        .wrapping_add(1 as std::ffi::c_int as usize)
                        .wrapping_sub(1 as std::ffi::c_int as usize)]
                        as std::ffi::c_uint
                    {
                        let Some(ptr) = self.alloc_units(i.wrapping_add(1)) else {
                            self.restore_model(c);
                            return;
                        };

                        let oldPtr = self
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
                        self.insert_node(oldPtr.cast(), i);
                        c.as_mut().union4.stats =
                            ptr.cast().offset_from(self.memory_ptr) as std::ffi::c_long as u32;
                    }
                }
                sum = c.as_ref().union2.summ_freq as u32;
                sum = sum.wrapping_add(
                    ((3 as std::ffi::c_int as std::ffi::c_uint)
                        .wrapping_mul(ns1)
                        .wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint)
                        < ns) as std::ffi::c_int as std::ffi::c_uint,
                );
            } else {
                let Some(s_0) = self.alloc_units(0) else {
                    self.restore_model(c);
                    return;
                };
                let mut s_0 = s_0.cast::<State>();

                let mut freq: std::ffi::c_uint = c.as_ref().union2.state2.freq as std::ffi::c_uint;
                s_0.as_mut().symbol = c.as_ref().union2.state2.symbol;
                s_0.as_mut().successor_0 = c.as_ref().union4.state4.successor_0;
                s_0.as_mut().successor_1 = c.as_ref().union4.state4.successor_1;
                c.as_mut().union4.stats =
                    s_0.cast().offset_from(self.memory_ptr) as std::ffi::c_long as u32;
                if freq
                    < (124 as std::ffi::c_int / 4 as std::ffi::c_int - 1 as std::ffi::c_int)
                        as std::ffi::c_uint
                {
                    freq <<= 1 as std::ffi::c_int;
                } else {
                    freq = (124 as std::ffi::c_int - 4 as std::ffi::c_int) as std::ffi::c_uint;
                }
                s_0.as_mut().freq = freq as u8;
                sum = freq.wrapping_add(self.init_esc).wrapping_add(
                    (ns > 2 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int
                        as std::ffi::c_uint,
                );
            }
            let mut s_1 = (self
                .memory_ptr
                .offset(c.as_ref().union4.stats as isize)
                .cast::<State>())
            .offset(ns1 as isize)
            .offset(1 as std::ffi::c_int as isize);
            let mut cf: u32 =
                2 as std::ffi::c_int as u32 * sum.wrapping_add(6 as std::ffi::c_int as u32) * fFreq;
            let sf: u32 = s0.wrapping_add(sum);
            s_1.as_mut().symbol = fSymbol;
            c.as_mut().num_stats = ns1.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint) as u8;
            s_1.as_mut().set_successor(maxSuccessor);
            c.as_mut().flags =
                (c.as_ref().flags as std::ffi::c_int | flag as std::ffi::c_int) as u8;
            if cf < 6 as std::ffi::c_int as u32 * sf {
                cf = (1 as std::ffi::c_int as std::ffi::c_uint)
                    .wrapping_add((cf > sf) as std::ffi::c_int as std::ffi::c_uint)
                    .wrapping_add(
                        (cf >= 4 as std::ffi::c_int as u32 * sf) as std::ffi::c_int
                            as std::ffi::c_uint,
                    );
                sum = sum.wrapping_add(4 as std::ffi::c_int as u32);
            } else {
                cf = (4 as std::ffi::c_int as std::ffi::c_uint)
                    .wrapping_add(
                        (cf > 9 as std::ffi::c_int as u32 * sf) as std::ffi::c_int
                            as std::ffi::c_uint,
                    )
                    .wrapping_add(
                        (cf > 12 as std::ffi::c_int as u32 * sf) as std::ffi::c_int
                            as std::ffi::c_uint,
                    )
                    .wrapping_add(
                        (cf > 15 as std::ffi::c_int as u32 * sf) as std::ffi::c_int
                            as std::ffi::c_uint,
                    );
                sum = sum.wrapping_add(cf);
            }
            c.as_mut().union2.summ_freq = sum as u16;
            s_1.as_mut().freq = cf as u8;
            c = self.memory_ptr.offset(c.as_ref().suffix as isize).cast();
        }
        self.min_context = self.memory_ptr.offset(minSuccessor as isize).cast();
        self.max_context = self.min_context;
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
        i = self.min_context.as_ref().num_stats as std::ffi::c_uint;
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
            mc.as_mut().num_stats = numStatsNew as u8;
            n0 = numStats.wrapping_add(2 as std::ffi::c_int as std::ffi::c_uint)
                >> 1 as std::ffi::c_int;
            if numStatsNew == 0 as std::ffi::c_int as std::ffi::c_uint {
                let mut freq_0: std::ffi::c_uint = (2 as std::ffi::c_int as std::ffi::c_uint)
                    .wrapping_mul(stats.as_ref().freq as std::ffi::c_uint)
                    .wrapping_add(escFreq)
                    .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint)
                    .wrapping_div(escFreq);
                if freq_0 > (124 as std::ffi::c_int / 3 as std::ffi::c_int) as std::ffi::c_uint {
                    freq_0 = (124 as std::ffi::c_int / 3 as std::ffi::c_int) as std::ffi::c_uint;
                }
                mc.as_mut().flags = ((mc.as_ref().flags as std::ffi::c_int
                    & (1 as std::ffi::c_int) << 4 as std::ffi::c_int)
                    as std::ffi::c_uint)
                    .wrapping_add(
                        (stats.as_ref().symbol as std::ffi::c_uint)
                            .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
                            >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
                            & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint,
                    ) as u8;
                s = NonNull::new_unchecked(addr_of_mut!(mc.as_mut().union2)).cast();
                *s.as_mut() = *stats.as_ref();
                s.as_mut().freq = freq_0 as u8;
                self.found_state = s;
                self.insert_node(
                    stats.cast(),
                    self.units2index
                        [(n0 as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
                        as std::ffi::c_uint,
                );
                return;
            }
            n1 = numStatsNew.wrapping_add(2 as std::ffi::c_int as std::ffi::c_uint)
                >> 1 as std::ffi::c_int;
            if n0 != n1 {
                mc.as_mut().union4.stats = self
                    .shrink_units(stats.cast(), n0, n1)
                    .offset_from(self.memory_ptr)
                    as std::ffi::c_long as u32;
            }
        }
        let mut mc_0 = self.min_context;
        mc_0.as_mut().union2.summ_freq = sumFreq
            .wrapping_add(escFreq)
            .wrapping_sub(escFreq >> 1 as std::ffi::c_int)
            as u16;
        mc_0.as_mut().flags = (mc_0.as_ref().flags as std::ffi::c_int
            | (1 as std::ffi::c_int) << 2 as std::ffi::c_int) as u8;
        self.found_state = self
            .memory_ptr
            .offset(mc_0.as_ref().union4.stats as isize)
            .cast();
    }

    pub unsafe fn make_esc_freq(
        &mut self,
        numMasked1: std::ffi::c_uint,
        escFreq: &mut u32,
    ) -> *mut See {
        let mut see: *mut See = 0 as *mut See;
        let mut mc = self.min_context;
        let numStats: std::ffi::c_uint = mc.as_ref().num_stats as std::ffi::c_uint;
        if numStats != 0xFF as std::ffi::c_int as std::ffi::c_uint {
            see = (self.see[(self.ns2index
                [(numStats as usize).wrapping_add(2 as std::ffi::c_int as usize) as usize]
                as std::ffi::c_uint as usize)
                .wrapping_sub(3 as std::ffi::c_int as usize) as usize])
                .as_mut_ptr()
                .offset(
                    (mc.as_ref().union2.summ_freq as std::ffi::c_uint
                        > (11 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(
                            numStats.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint),
                        )) as std::ffi::c_int as isize,
                )
                .offset(
                    (2 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(
                        ((2 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(numStats)
                            < (self
                                .memory_ptr
                                .offset(mc.as_ref().suffix as isize)
                                .cast::<Context>()
                                .as_ref()
                                .num_stats as std::ffi::c_uint)
                                .wrapping_add(numMasked1))
                            as std::ffi::c_int as std::ffi::c_uint,
                    ) as isize,
                )
                .offset(mc.as_ref().flags as std::ffi::c_int as isize);
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
        if self.order_fall == 0 as std::ffi::c_int as std::ffi::c_uint
            && c.cast() >= self.units_start
        {
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
        if freq > s.offset(-(1 as std::ffi::c_int) as isize).as_ref().freq as std::ffi::c_uint {
            swap(s.as_mut(), s.offset(-1).as_mut());
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
            >= summFreq) as std::ffi::c_int as std::ffi::c_uint;
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

impl<R: Read> PPMd8<RangeDecoder<R>> {
    pub(crate) fn new_decoder(
        reader: R,
        mem_size: u32,
        max_order: u32,
        restore_method: RestoreMethod,
    ) -> Result<Self, Error> {
        let range_decoder = RangeDecoder::new(reader)?;
        Self::construct(range_decoder, mem_size, max_order, restore_method)
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

impl<W: Write> PPMd8<RangeEncoder<W>> {
    pub(crate) fn new_encoder(
        writer: W,
        mem_size: u32,
        max_order: u32,
        restore_method: RestoreMethod,
    ) -> Result<Self, Error> {
        let range_encoder = RangeEncoder::new(writer);
        Self::construct(range_encoder, mem_size, max_order, restore_method)
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

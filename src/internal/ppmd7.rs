mod decoder;
mod encoder;
mod range_coding;

use std::{
    alloc::{alloc_zeroed, dealloc, Layout},
    io::{Read, Write},
    mem::ManuallyDrop,
    ptr::{null_mut, NonNull},
};

pub(crate) use decoder::*;
pub(crate) use encoder::*;
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
    min_context: *mut Context,
    max_context: *mut Context,
    found_state: *mut State,
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
    base: *mut u8,
    lo_unit: *mut u8,
    hi_unit: *mut u8,
    text: *mut u8,
    units_start: *mut u8,
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
            min_context: null_mut(),
            max_context: null_mut(),
            found_state: null_mut(),
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
            base: memory_ptr.as_ptr(),
            lo_unit: null_mut(),
            hi_unit: null_mut(),
            text: null_mut(),
            units_start: null_mut(),
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

        unsafe { restart_model(&mut ppmd) };

        Ok(ppmd)
    }
}

unsafe fn insert_node<RC>(p: *mut PPMd7<RC>, node: *mut std::ffi::c_void, indx: std::ffi::c_uint) {
    *(node as *mut u32) = (*p).free_list[indx as usize];
    (*p).free_list[indx as usize] =
        (node as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32;
}

unsafe fn remove_node<RC>(p: *mut PPMd7<RC>, indx: std::ffi::c_uint) -> *mut std::ffi::c_void {
    let node: *mut u32 = ((*p).base).offset((*p).free_list[indx as usize] as isize)
        as *mut std::ffi::c_void as *mut u32;
    (*p).free_list[indx as usize] = *node;
    return node as *mut std::ffi::c_void;
}

unsafe fn split_block<RC>(
    p: *mut PPMd7<RC>,
    mut ptr: *mut std::ffi::c_void,
    oldIndx: std::ffi::c_uint,
    newIndx: std::ffi::c_uint,
) {
    let mut i: std::ffi::c_uint = 0;
    let nu: std::ffi::c_uint = ((*p).index2units[oldIndx as usize] as std::ffi::c_uint)
        .wrapping_sub((*p).index2units[newIndx as usize] as std::ffi::c_uint);
    ptr = (ptr as *mut u8).offset(
        ((*p).index2units[newIndx as usize] as std::ffi::c_uint * 12 as std::ffi::c_int as u32)
            as isize,
    ) as *mut std::ffi::c_void;
    i = (*p).units2index[(nu as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
        as std::ffi::c_uint;
    if (*p).index2units[i as usize] as std::ffi::c_uint != nu {
        i = i.wrapping_sub(1);
        let k: std::ffi::c_uint = (*p).index2units[i as usize] as std::ffi::c_uint;
        insert_node(
            p,
            (ptr as *mut u8).offset((k * 12 as std::ffi::c_int as u32) as isize)
                as *mut std::ffi::c_void,
            nu.wrapping_sub(k)
                .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint),
        );
    }
    insert_node(p, ptr, i);
}
unsafe fn glue_free_blocks<RC>(p: *mut PPMd7<RC>) {
    let mut head: u32 = 0;
    let mut n: u32 = 0 as std::ffi::c_int as u32;
    (*p).glue_count = 255 as std::ffi::c_int as u32;
    if (*p).lo_unit != (*p).hi_unit {
        (*((*p).lo_unit as *mut std::ffi::c_void as *mut Node)).stamp = 1 as std::ffi::c_int as u16;
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
        let nu: u16 = (*p).index2units[i as usize] as u16;
        let mut next: u32 = (*p).free_list[i as usize];
        (*p).free_list[i as usize] = 0 as std::ffi::c_int as u32;
        while next != 0 as std::ffi::c_int as u32 {
            let un: *mut NodeUnion = ((*p).base).offset(next as isize) as *mut std::ffi::c_void
                as *mut Node as *mut NodeUnion;
            let tmp: u32 = next;
            next = (*un).next_ref;
            (*un).node.stamp = 0 as std::ffi::c_int as u16;
            (*un).node.nu = nu;
            (*un).node.next = n;
            n = tmp;
        }
        i = i.wrapping_add(1);
        i;
    }
    head = n;
    let mut prev: *mut u32 = &mut head;
    while n != 0 {
        let node: *mut Node = ((*p).base).offset(n as isize) as *mut std::ffi::c_void as *mut Node;
        let mut nu_0: u32 = (*node).nu as u32;
        n = (*node).next;
        if nu_0 == 0 as std::ffi::c_int as u32 {
            *prev = n;
        } else {
            prev = &mut (*node).next;
            loop {
                let node2: *mut Node = node.offset(nu_0 as isize);
                nu_0 = nu_0.wrapping_add((*node2).nu as u32);
                if (*node2).stamp as std::ffi::c_int != 0 as std::ffi::c_int
                    || nu_0 >= 0x10000 as std::ffi::c_int as u32
                {
                    break;
                }
                (*node).nu = nu_0 as u16;
                (*node2).nu = 0 as std::ffi::c_int as u16;
            }
        }
    }
    n = head;
    while n != 0 as std::ffi::c_int as u32 {
        let mut node_0: *mut Node =
            ((*p).base).offset(n as isize) as *mut std::ffi::c_void as *mut Node;
        let mut nu_1: u32 = (*node_0).nu as u32;
        let mut i_0: std::ffi::c_uint = 0;
        n = (*node_0).next;
        if nu_1 == 0 as std::ffi::c_int as u32 {
            continue;
        }
        while nu_1 > 128 as std::ffi::c_int as u32 {
            insert_node(
                p,
                node_0 as *mut std::ffi::c_void,
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
        i_0 = (*p).units2index[(nu_1 as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
            as std::ffi::c_uint;
        if (*p).index2units[i_0 as usize] as std::ffi::c_uint != nu_1 {
            i_0 = i_0.wrapping_sub(1);
            let k: std::ffi::c_uint = (*p).index2units[i_0 as usize] as std::ffi::c_uint;
            insert_node(
                p,
                node_0.offset(k as isize) as *mut std::ffi::c_void,
                nu_1.wrapping_sub(k)
                    .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint),
            );
        }
        insert_node(p, node_0 as *mut std::ffi::c_void, i_0);
    }
}
#[inline(never)]
unsafe fn alloc_units_rare<RC>(p: *mut PPMd7<RC>, indx: std::ffi::c_uint) -> *mut std::ffi::c_void {
    let mut i: std::ffi::c_uint = 0;
    if (*p).glue_count == 0 as std::ffi::c_int as u32 {
        glue_free_blocks(p);
        if (*p).free_list[indx as usize] != 0 as std::ffi::c_int as u32 {
            return remove_node(p, indx);
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
            let numBytes: u32 =
                (*p).index2units[indx as usize] as std::ffi::c_uint * 12 as std::ffi::c_int as u32;
            let us: *mut u8 = (*p).units_start;
            (*p).glue_count = ((*p).glue_count).wrapping_sub(1);
            (*p).glue_count;
            return (if us.offset_from((*p).text) as std::ffi::c_long as u32 > numBytes {
                (*p).units_start = us.offset(-(numBytes as isize));
                (*p).units_start
            } else {
                0 as *mut u8
            }) as *mut std::ffi::c_void;
        }
        if !((*p).free_list[i as usize] == 0 as std::ffi::c_int as u32) {
            break;
        }
    }
    let block: *mut std::ffi::c_void = remove_node(p, i);
    split_block(p, block, i, indx);
    return block;
}
unsafe fn alloc_units<RC>(p: *mut PPMd7<RC>, indx: std::ffi::c_uint) -> *mut std::ffi::c_void {
    if (*p).free_list[indx as usize] != 0 as std::ffi::c_int as u32 {
        return remove_node(p, indx);
    }
    let numBytes: u32 =
        (*p).index2units[indx as usize] as std::ffi::c_uint * 12 as std::ffi::c_int as u32;
    let lo: *mut u8 = (*p).lo_unit;
    if ((*p).hi_unit).offset_from(lo) as std::ffi::c_long as u32 >= numBytes {
        (*p).lo_unit = lo.offset(numBytes as isize);
        return lo as *mut std::ffi::c_void;
    }
    return alloc_units_rare(p, indx);
}
unsafe fn set_successor(p: *mut State, v: u32) {
    (*p).successor_0 = v as u16;
    (*p).successor_1 = (v >> 16 as std::ffi::c_int) as u16;
}
#[inline(never)]
unsafe fn restart_model<RC>(p: *mut PPMd7<RC>) {
    let mut i: std::ffi::c_uint = 0;
    let mut k: std::ffi::c_uint = 0;
    ((*p).free_list).as_mut_ptr().write_bytes(0, 38);
    (*p).text = ((*p).base).offset((*p).align_offset as isize);
    (*p).hi_unit = ((*p).text).offset((*p).size as isize);
    (*p).units_start = ((*p).hi_unit).offset(
        -(((*p).size / 8 as std::ffi::c_int as u32 / 12 as std::ffi::c_int as u32
            * 7 as std::ffi::c_int as u32
            * 12 as std::ffi::c_int as u32) as isize),
    );
    (*p).lo_unit = (*p).units_start;
    (*p).glue_count = 0 as std::ffi::c_int as u32;
    (*p).order_fall = (*p).max_order;
    (*p).init_rl = -((if (*p).max_order < 12 as std::ffi::c_int as std::ffi::c_uint {
        (*p).max_order
    } else {
        12 as std::ffi::c_int as std::ffi::c_uint
    }) as i32)
        - 1 as std::ffi::c_int;
    (*p).run_length = (*p).init_rl;
    (*p).prev_success = 0 as std::ffi::c_int as std::ffi::c_uint;
    (*p).hi_unit = ((*p).hi_unit).offset(-(12 as std::ffi::c_int as isize));
    let mc: *mut Context = (*p).hi_unit as *mut std::ffi::c_void as *mut Context;
    let mut s: *mut State = (*p).lo_unit as *mut State;
    (*p).lo_unit = ((*p).lo_unit).offset(
        ((256 as std::ffi::c_int / 2 as std::ffi::c_int) as u32 * 12 as std::ffi::c_int as u32)
            as isize,
    );
    (*p).min_context = mc;
    (*p).max_context = (*p).min_context;
    (*p).found_state = s;
    (*mc).num_stats = 256 as std::ffi::c_int as u16;
    (*mc).union2.summ_freq = (256 as std::ffi::c_int + 1 as std::ffi::c_int) as u16;
    (*mc).union4.stats = (s as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32;
    (*mc).suffix = 0 as std::ffi::c_int as u32;
    i = 0 as std::ffi::c_int as std::ffi::c_uint;
    while i < 256 as std::ffi::c_int as std::ffi::c_uint {
        (*s).symbol = i as u8;
        (*s).freq = 1 as std::ffi::c_int as u8;
        set_successor(s, 0 as std::ffi::c_int as u32);
        i = i.wrapping_add(1);
        i;
        s = s.offset(1);
        s;
    }
    i = 0 as std::ffi::c_int as std::ffi::c_uint;
    while i < 128 as std::ffi::c_int as std::ffi::c_uint {
        k = 0 as std::ffi::c_int as std::ffi::c_uint;
        while k < 8 as std::ffi::c_int as std::ffi::c_uint {
            let mut m: std::ffi::c_uint = 0;
            let dest: *mut u16 = ((*p).bin_summ[i as usize]).as_mut_ptr().offset(k as isize);
            let val: u16 = (((1 as std::ffi::c_int) << 7 as std::ffi::c_int + 7 as std::ffi::c_int)
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
            k;
        }
        i = i.wrapping_add(1);
        i;
    }
    i = 0 as std::ffi::c_int as std::ffi::c_uint;
    while i < 25 as std::ffi::c_int as std::ffi::c_uint {
        let mut s_0: *mut See = ((*p).see[i as usize]).as_mut_ptr();
        let summ: std::ffi::c_uint = (5 as std::ffi::c_int as std::ffi::c_uint)
            .wrapping_mul(i)
            .wrapping_add(10 as std::ffi::c_int as std::ffi::c_uint)
            << 7 as std::ffi::c_int - 4 as std::ffi::c_int;
        k = 0 as std::ffi::c_int as std::ffi::c_uint;
        while k < 16 as std::ffi::c_int as std::ffi::c_uint {
            (*s_0).summ = summ as u16;
            (*s_0).shift = (7 as std::ffi::c_int - 4 as std::ffi::c_int) as u8;
            (*s_0).count = 4 as std::ffi::c_int as u8;
            k = k.wrapping_add(1);
            k;
            s_0 = s_0.offset(1);
            s_0;
        }
        i = i.wrapping_add(1);
        i;
    }
    (*p).dummy_see.summ = 0 as std::ffi::c_int as u16;
    (*p).dummy_see.shift = 7 as std::ffi::c_int as u8;
    (*p).dummy_see.count = 64 as std::ffi::c_int as u8;
}

#[inline(never)]
unsafe fn create_successors<RC>(p: *mut PPMd7<RC>) -> *mut Context {
    let mut c: *mut Context = (*p).min_context;
    let mut upBranch: u32 = (*(*p).found_state).successor_0 as u32
        | ((*(*p).found_state).successor_1 as u32) << 16 as std::ffi::c_int;
    let mut newSym: u8 = 0;
    let mut newFreq: u8 = 0;
    let mut numPs: std::ffi::c_uint = 0 as std::ffi::c_int as std::ffi::c_uint;
    let mut ps: [*mut State; 64] = [0 as *mut State; 64];
    if (*p).order_fall != 0 as std::ffi::c_int as std::ffi::c_uint {
        let fresh1 = numPs;
        numPs = numPs.wrapping_add(1);
        ps[fresh1 as usize] = (*p).found_state;
    }
    while (*c).suffix != 0 {
        let mut successor: u32 = 0;
        let mut s: *mut State = 0 as *mut State;
        c = ((*p).base).offset((*c).suffix as isize) as *mut std::ffi::c_void as *mut Context;
        if (*c).num_stats as std::ffi::c_int != 1 as std::ffi::c_int {
            let sym: u8 = (*(*p).found_state).symbol;
            s = ((*p).base).offset((*c).union4.stats as isize) as *mut std::ffi::c_void
                as *mut State;
            while (*s).symbol as std::ffi::c_int != sym as std::ffi::c_int {
                s = s.offset(1);
                s;
            }
        } else {
            s = &mut (*c).union2 as *mut Union2 as *mut State;
        }
        successor = (*s).successor_0 as u32 | ((*s).successor_1 as u32) << 16 as std::ffi::c_int;
        if successor != upBranch {
            c = ((*p).base).offset(successor as isize) as *mut std::ffi::c_void as *mut Context;
            if numPs == 0 as std::ffi::c_int as std::ffi::c_uint {
                return c;
            }
            break;
        } else {
            let fresh2 = numPs;
            numPs = numPs.wrapping_add(1);
            ps[fresh2 as usize] = s;
        }
    }
    newSym = *(((*p).base).offset(upBranch as isize) as *mut std::ffi::c_void as *const u8);
    upBranch = upBranch.wrapping_add(1);
    upBranch;
    if (*c).num_stats as std::ffi::c_int == 1 as std::ffi::c_int {
        newFreq = (*(&mut (*c).union2 as *mut Union2 as *mut State)).freq;
    } else {
        let mut cf: u32 = 0;
        let mut s0: u32 = 0;
        let mut s_0: *mut State = 0 as *mut State;
        s_0 = ((*p).base).offset((*c).union4.stats as isize) as *mut std::ffi::c_void as *mut State;
        while (*s_0).symbol as std::ffi::c_int != newSym as std::ffi::c_int {
            s_0 = s_0.offset(1);
            s_0;
        }
        cf = ((*s_0).freq as u32).wrapping_sub(1 as std::ffi::c_int as u32);
        s0 = ((*c).union2.summ_freq as u32)
            .wrapping_sub((*c).num_stats as u32)
            .wrapping_sub(cf);
        newFreq =
            (1 as std::ffi::c_int as u32).wrapping_add(if 2 as std::ffi::c_int as u32 * cf <= s0 {
                (5 as std::ffi::c_int as u32 * cf > s0) as std::ffi::c_int as u32
            } else {
                ((2 as std::ffi::c_int as u32 * cf)
                    .wrapping_add(s0)
                    .wrapping_sub(1 as std::ffi::c_int as u32)
                    / (2 as std::ffi::c_int as u32 * s0))
                    .wrapping_add(1 as std::ffi::c_int as u32)
            }) as u8;
    }
    loop {
        let mut c1: *mut Context = 0 as *mut Context;
        if (*p).hi_unit != (*p).lo_unit {
            (*p).hi_unit = ((*p).hi_unit).offset(-(12 as std::ffi::c_int as isize));
            c1 = (*p).hi_unit as *mut std::ffi::c_void as *mut Context;
        } else if (*p).free_list[0 as std::ffi::c_int as usize] != 0 as std::ffi::c_int as u32 {
            c1 = remove_node(p, 0 as std::ffi::c_int as std::ffi::c_uint) as *mut Context;
        } else {
            c1 = alloc_units_rare(p, 0 as std::ffi::c_int as std::ffi::c_uint) as *mut Context;
            if c1.is_null() {
                return 0 as *mut Context;
            }
        }
        (*c1).num_stats = 1 as std::ffi::c_int as u16;
        (*(&mut (*c1).union2 as *mut Union2 as *mut State)).symbol = newSym;
        (*(&mut (*c1).union2 as *mut Union2 as *mut State)).freq = newFreq;
        set_successor(&mut (*c1).union2 as *mut Union2 as *mut State, upBranch);
        (*c1).suffix = (c as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32;
        numPs = numPs.wrapping_sub(1);
        set_successor(
            ps[numPs as usize],
            (c1 as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32,
        );
        c = c1;
        if !(numPs != 0 as std::ffi::c_int as std::ffi::c_uint) {
            break;
        }
    }
    return c;
}

#[inline(never)]
pub unsafe fn update_model<RC>(p: *mut PPMd7<RC>) {
    let mut maxSuccessor: u32 = 0;
    let mut minSuccessor: u32 = 0;
    let mut c: *mut Context = 0 as *mut Context;
    let mut mc: *mut Context = 0 as *mut Context;
    let mut s0: std::ffi::c_uint = 0;
    let mut ns: std::ffi::c_uint = 0;
    if ((*(*p).found_state).freq as std::ffi::c_int) < 124 as std::ffi::c_int / 4 as std::ffi::c_int
        && (*(*p).min_context).suffix != 0 as std::ffi::c_int as u32
    {
        c = ((*p).base).offset((*(*p).min_context).suffix as isize) as *mut std::ffi::c_void
            as *mut Context;
        if (*c).num_stats as std::ffi::c_int == 1 as std::ffi::c_int {
            let s: *mut State = &mut (*c).union2 as *mut Union2 as *mut State;
            if ((*s).freq as std::ffi::c_int) < 32 as std::ffi::c_int {
                (*s).freq = ((*s).freq).wrapping_add(1);
                (*s).freq;
            }
        } else {
            let mut s_0: *mut State = ((*p).base).offset((*c).union4.stats as isize)
                as *mut std::ffi::c_void as *mut State;
            let sym: u8 = (*(*p).found_state).symbol;
            if (*s_0).symbol as std::ffi::c_int != sym as std::ffi::c_int {
                loop {
                    s_0 = s_0.offset(1);
                    s_0;
                    if !((*s_0).symbol as std::ffi::c_int != sym as std::ffi::c_int) {
                        break;
                    }
                }
                if (*s_0.offset(0 as std::ffi::c_int as isize)).freq as std::ffi::c_int
                    >= (*s_0.offset(-(1 as std::ffi::c_int) as isize)).freq as std::ffi::c_int
                {
                    let tmp: State = *s_0.offset(0 as std::ffi::c_int as isize);
                    *s_0.offset(0 as std::ffi::c_int as isize) =
                        *s_0.offset(-(1 as std::ffi::c_int) as isize);
                    *s_0.offset(-(1 as std::ffi::c_int) as isize) = tmp;
                    s_0 = s_0.offset(-1);
                    s_0;
                }
            }
            if ((*s_0).freq as std::ffi::c_int) < 124 as std::ffi::c_int - 9 as std::ffi::c_int {
                (*s_0).freq = ((*s_0).freq as std::ffi::c_int + 2 as std::ffi::c_int) as u8;
                (*c).union2.summ_freq =
                    ((*c).union2.summ_freq as std::ffi::c_int + 2 as std::ffi::c_int) as u16;
            }
        }
    }
    if (*p).order_fall == 0 as std::ffi::c_int as std::ffi::c_uint {
        (*p).min_context = create_successors(p);
        (*p).max_context = (*p).min_context;
        if ((*p).min_context).is_null() {
            restart_model(p);
            return;
        }
        set_successor(
            (*p).found_state,
            ((*p).min_context as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32,
        );
        return;
    }
    let mut text: *mut u8 = (*p).text;
    let fresh3 = text;
    text = text.offset(1);
    *fresh3 = (*(*p).found_state).symbol;
    (*p).text = text;
    if text >= (*p).units_start {
        restart_model(p);
        return;
    }
    maxSuccessor = text.offset_from((*p).base) as std::ffi::c_long as u32;
    minSuccessor = (*(*p).found_state).successor_0 as u32
        | ((*(*p).found_state).successor_1 as u32) << 16 as std::ffi::c_int;
    if minSuccessor != 0 {
        if minSuccessor <= maxSuccessor {
            let cs: *mut Context = create_successors(p);
            if cs.is_null() {
                restart_model(p);
                return;
            }
            minSuccessor = (cs as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32;
        }
        (*p).order_fall = ((*p).order_fall).wrapping_sub(1);
        if (*p).order_fall == 0 as std::ffi::c_int as std::ffi::c_uint {
            maxSuccessor = minSuccessor;
            (*p).text = ((*p).text)
                .offset(-(((*p).max_context != (*p).min_context) as std::ffi::c_int as isize));
        }
    } else {
        set_successor((*p).found_state, maxSuccessor);
        minSuccessor =
            ((*p).min_context as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32;
    }
    mc = (*p).min_context;
    c = (*p).max_context;
    (*p).min_context =
        ((*p).base).offset(minSuccessor as isize) as *mut std::ffi::c_void as *mut Context;
    (*p).max_context = (*p).min_context;
    if c == mc {
        return;
    }
    ns = (*mc).num_stats as std::ffi::c_uint;
    s0 = ((*mc).union2.summ_freq as std::ffi::c_uint)
        .wrapping_sub(ns)
        .wrapping_sub(
            ((*(*p).found_state).freq as std::ffi::c_uint)
                .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint),
        );
    loop {
        let mut ns1: std::ffi::c_uint = 0;
        let mut sum: u32 = 0;
        ns1 = (*c).num_stats as std::ffi::c_uint;
        if ns1 != 1 as std::ffi::c_int as std::ffi::c_uint {
            if ns1 & 1 as std::ffi::c_int as std::ffi::c_uint
                == 0 as std::ffi::c_int as std::ffi::c_uint
            {
                let oldNU: std::ffi::c_uint = ns1 >> 1 as std::ffi::c_int;
                let i: std::ffi::c_uint = (*p).units2index
                    [(oldNU as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
                    as std::ffi::c_uint;
                if i != (*p).units2index[(oldNU as usize)
                    .wrapping_add(1 as std::ffi::c_int as usize)
                    .wrapping_sub(1 as std::ffi::c_int as usize)
                    as usize] as std::ffi::c_uint
                {
                    let ptr: *mut std::ffi::c_void =
                        alloc_units(p, i.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint));
                    let mut oldPtr: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
                    if ptr.is_null() {
                        restart_model(p);
                        return;
                    }
                    oldPtr = ((*p).base).offset((*c).union4.stats as isize) as *mut std::ffi::c_void
                        as *mut State as *mut std::ffi::c_void;
                    let mut d: *mut u32 = ptr as *mut u32;
                    let mut z: *const u32 = oldPtr as *const u32;
                    let mut n: std::ffi::c_uint = oldNU;
                    loop {
                        *d.offset(0 as std::ffi::c_int as isize) =
                            *z.offset(0 as std::ffi::c_int as isize);
                        *d.offset(1 as std::ffi::c_int as isize) =
                            *z.offset(1 as std::ffi::c_int as isize);
                        *d.offset(2 as std::ffi::c_int as isize) =
                            *z.offset(2 as std::ffi::c_int as isize);
                        z = z.offset(3 as std::ffi::c_int as isize);
                        d = d.offset(3 as std::ffi::c_int as isize);
                        n = n.wrapping_sub(1);
                        if !(n != 0) {
                            break;
                        }
                    }
                    insert_node(p, oldPtr, i);
                    (*c).union4.stats =
                        (ptr as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32;
                }
            }
            sum = (*c).union2.summ_freq as u32;
            sum = sum.wrapping_add(
                (((2 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(ns1) < ns)
                    as std::ffi::c_int as std::ffi::c_uint)
                    .wrapping_add((2 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(
                        ((4 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(ns1) <= ns)
                            as std::ffi::c_int as std::ffi::c_uint
                            & (sum <= (8 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(ns1))
                                as std::ffi::c_int
                                as std::ffi::c_uint,
                    )),
            );
        } else {
            let s_1: *mut State =
                alloc_units(p, 0 as std::ffi::c_int as std::ffi::c_uint) as *mut State;
            if s_1.is_null() {
                restart_model(p);
                return;
            }
            let mut freq: std::ffi::c_uint = (*c).union2.state2.freq as std::ffi::c_uint;
            (*s_1).symbol = (*c).union2.state2.symbol;
            (*s_1).successor_0 = (*c).union4.state4.successor_0;
            (*s_1).successor_1 = (*c).union4.state4.successor_1;
            (*c).union4.stats = (s_1 as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32;
            if freq
                < (124 as std::ffi::c_int / 4 as std::ffi::c_int - 1 as std::ffi::c_int)
                    as std::ffi::c_uint
            {
                freq <<= 1 as std::ffi::c_int;
            } else {
                freq = (124 as std::ffi::c_int - 4 as std::ffi::c_int) as std::ffi::c_uint;
            }
            (*s_1).freq = freq as u8;
            sum = freq.wrapping_add((*p).init_esc).wrapping_add(
                (ns > 3 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int
                    as std::ffi::c_uint,
            );
        }
        let s_2: *mut State = (((*p).base).offset((*c).union4.stats as isize)
            as *mut std::ffi::c_void as *mut State)
            .offset(ns1 as isize);
        let mut cf: u32 = 2 as std::ffi::c_int as u32
            * sum.wrapping_add(6 as std::ffi::c_int as u32)
            * (*(*p).found_state).freq as u32;
        let sf: u32 = s0.wrapping_add(sum);
        (*s_2).symbol = (*(*p).found_state).symbol;
        (*c).num_stats = ns1.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint) as u16;
        set_successor(s_2, maxSuccessor);
        if cf < 6 as std::ffi::c_int as u32 * sf {
            cf = (1 as std::ffi::c_int as u32)
                .wrapping_add((cf > sf) as std::ffi::c_int as u32)
                .wrapping_add((cf >= 4 as std::ffi::c_int as u32 * sf) as std::ffi::c_int as u32);
            sum = sum.wrapping_add(3 as std::ffi::c_int as u32);
        } else {
            cf = (4 as std::ffi::c_int as u32)
                .wrapping_add((cf >= 9 as std::ffi::c_int as u32 * sf) as std::ffi::c_int as u32)
                .wrapping_add((cf >= 12 as std::ffi::c_int as u32 * sf) as std::ffi::c_int as u32)
                .wrapping_add((cf >= 15 as std::ffi::c_int as u32 * sf) as std::ffi::c_int as u32);
            sum = sum.wrapping_add(cf);
        }
        (*c).union2.summ_freq = sum as u16;
        (*s_2).freq = cf as u8;
        c = ((*p).base).offset((*c).suffix as isize) as *mut std::ffi::c_void as *mut Context;
        if !(c != mc) {
            break;
        }
    }
}

#[inline(never)]
unsafe fn rescale<RC>(p: *mut PPMd7<RC>) {
    let mut i: std::ffi::c_uint = 0;
    let mut adder: std::ffi::c_uint = 0;
    let mut sumFreq: std::ffi::c_uint = 0;
    let mut escFreq: std::ffi::c_uint = 0;
    let stats: *mut State = ((*p).base).offset((*(*p).min_context).union4.stats as isize)
        as *mut std::ffi::c_void as *mut State;
    let mut s: *mut State = (*p).found_state;
    if s != stats {
        let tmp: State = *s;
        loop {
            *s.offset(0 as std::ffi::c_int as isize) = *s.offset(-(1 as std::ffi::c_int) as isize);
            s = s.offset(-1);
            if !(s != stats) {
                break;
            }
        }
        *s = tmp;
    }
    sumFreq = (*s).freq as std::ffi::c_uint;
    escFreq = ((*(*p).min_context).union2.summ_freq as std::ffi::c_uint).wrapping_sub(sumFreq);
    adder = ((*p).order_fall != 0 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int
        as std::ffi::c_uint;
    sumFreq = sumFreq
        .wrapping_add(4 as std::ffi::c_int as std::ffi::c_uint)
        .wrapping_add(adder)
        >> 1 as std::ffi::c_int;
    i = ((*(*p).min_context).num_stats as std::ffi::c_uint)
        .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint);
    (*s).freq = sumFreq as u8;
    loop {
        s = s.offset(1);
        let mut freq: std::ffi::c_uint = (*s).freq as std::ffi::c_uint;
        escFreq = escFreq.wrapping_sub(freq);
        freq = freq.wrapping_add(adder) >> 1 as std::ffi::c_int;
        sumFreq = sumFreq.wrapping_add(freq);
        (*s).freq = freq as u8;
        if freq > (*s.offset(-(1 as std::ffi::c_int) as isize)).freq as std::ffi::c_uint {
            let tmp_0: State = *s;
            let mut s1: *mut State = s;
            loop {
                *s1.offset(0 as std::ffi::c_int as isize) =
                    *s1.offset(-(1 as std::ffi::c_int) as isize);
                s1 = s1.offset(-1);
                if !(s1 != stats
                    && freq
                        > (*s1.offset(-(1 as std::ffi::c_int) as isize)).freq as std::ffi::c_uint)
                {
                    break;
                }
            }
            *s1 = tmp_0;
        }
        i = i.wrapping_sub(1);
        if !(i != 0) {
            break;
        }
    }
    if (*s).freq as std::ffi::c_int == 0 as std::ffi::c_int {
        let mut mc: *mut Context = 0 as *mut Context;
        let mut numStats: std::ffi::c_uint = 0;
        let mut numStatsNew: std::ffi::c_uint = 0;
        let mut n0: std::ffi::c_uint = 0;
        let mut n1: std::ffi::c_uint = 0;
        i = 0 as std::ffi::c_int as std::ffi::c_uint;
        loop {
            i = i.wrapping_add(1);
            i;
            s = s.offset(-1);
            if !((*s).freq as std::ffi::c_int == 0 as std::ffi::c_int) {
                break;
            }
        }
        escFreq = escFreq.wrapping_add(i);
        mc = (*p).min_context;
        numStats = (*mc).num_stats as std::ffi::c_uint;
        numStatsNew = numStats.wrapping_sub(i);
        (*mc).num_stats = numStatsNew as u16;
        n0 =
            numStats.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint) >> 1 as std::ffi::c_int;
        if numStatsNew == 1 as std::ffi::c_int as std::ffi::c_uint {
            let mut freq_0: std::ffi::c_uint = (*stats).freq as std::ffi::c_uint;
            loop {
                escFreq >>= 1 as std::ffi::c_int;
                freq_0 = freq_0.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint)
                    >> 1 as std::ffi::c_int;
                if !(escFreq > 1 as std::ffi::c_int as std::ffi::c_uint) {
                    break;
                }
            }
            s = &mut (*mc).union2 as *mut Union2 as *mut State;
            *s = *stats;
            (*s).freq = freq_0 as u8;
            (*p).found_state = s;
            insert_node(
                p,
                stats as *mut std::ffi::c_void,
                (*p).units2index[(n0 as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
                    as std::ffi::c_uint,
            );
            return;
        }
        n1 = numStatsNew.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint)
            >> 1 as std::ffi::c_int;
        if n0 != n1 {
            let i0: std::ffi::c_uint = (*p).units2index
                [(n0 as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
                as std::ffi::c_uint;
            let i1: std::ffi::c_uint = (*p).units2index
                [(n1 as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
                as std::ffi::c_uint;
            if i0 != i1 {
                if (*p).free_list[i1 as usize] != 0 as std::ffi::c_int as u32 {
                    let ptr: *mut std::ffi::c_void = remove_node(p, i1);
                    (*(*p).min_context).union4.stats =
                        (ptr as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32;
                    let mut d: *mut u32 = ptr as *mut u32;
                    let mut z: *const u32 = stats as *const std::ffi::c_void as *const u32;
                    let mut n: std::ffi::c_uint = n1;
                    loop {
                        *d.offset(0 as std::ffi::c_int as isize) =
                            *z.offset(0 as std::ffi::c_int as isize);
                        *d.offset(1 as std::ffi::c_int as isize) =
                            *z.offset(1 as std::ffi::c_int as isize);
                        *d.offset(2 as std::ffi::c_int as isize) =
                            *z.offset(2 as std::ffi::c_int as isize);
                        z = z.offset(3 as std::ffi::c_int as isize);
                        d = d.offset(3 as std::ffi::c_int as isize);
                        n = n.wrapping_sub(1);
                        if !(n != 0) {
                            break;
                        }
                    }
                    insert_node(p, stats as *mut std::ffi::c_void, i0);
                } else {
                    split_block(p, stats as *mut std::ffi::c_void, i0, i1);
                }
            }
        }
    }
    let mc_0: *mut Context = (*p).min_context;
    (*mc_0).union2.summ_freq = sumFreq
        .wrapping_add(escFreq)
        .wrapping_sub(escFreq >> 1 as std::ffi::c_int) as u16;
    (*p).found_state =
        ((*p).base).offset((*mc_0).union4.stats as isize) as *mut std::ffi::c_void as *mut State;
}

pub unsafe fn make_esc_freq<RC>(
    p: *mut PPMd7<RC>,
    numMasked: std::ffi::c_uint,
    escFreq: *mut u32,
) -> *mut See {
    let mut see: *mut See = 0 as *mut See;
    let mc: *const Context = (*p).min_context;
    let numStats: std::ffi::c_uint = (*mc).num_stats as std::ffi::c_uint;
    if numStats != 256 as std::ffi::c_int as std::ffi::c_uint {
        let nonMasked: std::ffi::c_uint = numStats.wrapping_sub(numMasked);
        see = ((*p).see[(*p).ns2index
            [(nonMasked as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
            as std::ffi::c_uint as usize])
            .as_mut_ptr()
            .offset(
                (nonMasked
                    < ((*(((*p).base).offset((*mc).suffix as isize) as *mut std::ffi::c_void
                        as *mut Context))
                        .num_stats as std::ffi::c_uint)
                        .wrapping_sub(numStats)) as std::ffi::c_int as isize,
            )
            .offset((2 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(
                (((*mc).union2.summ_freq as std::ffi::c_uint)
                    < (11 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(numStats))
                    as std::ffi::c_int as std::ffi::c_uint,
            ) as isize)
            .offset(
                (4 as std::ffi::c_int as std::ffi::c_uint)
                    .wrapping_mul((numMasked > nonMasked) as std::ffi::c_int as std::ffi::c_uint)
                    as isize,
            )
            .offset((*p).hi_bits_flag as isize);
        let summ: std::ffi::c_uint = (*see).summ as std::ffi::c_uint;
        let r: std::ffi::c_uint = summ >> (*see).shift as std::ffi::c_int;
        (*see).summ = summ.wrapping_sub(r) as u16;
        *escFreq = r.wrapping_add(
            (r == 0 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int as std::ffi::c_uint,
        );
    } else {
        see = &mut (*p).dummy_see;
        *escFreq = 1 as std::ffi::c_int as u32;
    }
    return see;
}
unsafe fn next_context<RC>(p: *mut PPMd7<RC>) {
    let c: *mut Context = ((*p).base).offset(
        ((*(*p).found_state).successor_0 as u32
            | ((*(*p).found_state).successor_1 as u32) << 16 as std::ffi::c_int) as isize,
    ) as *mut std::ffi::c_void as *mut Context;
    if (*p).order_fall == 0 as std::ffi::c_int as std::ffi::c_uint
        && c as *const u8 > (*p).text as *const u8
    {
        (*p).min_context = c;
        (*p).max_context = (*p).min_context;
    } else {
        update_model(p);
    };
}

pub unsafe fn update1<RC>(p: *mut PPMd7<RC>) {
    let mut s: *mut State = (*p).found_state;
    let mut freq: std::ffi::c_uint = (*s).freq as std::ffi::c_uint;
    freq = freq.wrapping_add(4 as std::ffi::c_int as std::ffi::c_uint);
    (*(*p).min_context).union2.summ_freq =
        ((*(*p).min_context).union2.summ_freq as std::ffi::c_int + 4 as std::ffi::c_int) as u16;
    (*s).freq = freq as u8;
    if freq > (*s.offset(-(1 as std::ffi::c_int) as isize)).freq as std::ffi::c_uint {
        let tmp: State = *s.offset(0 as std::ffi::c_int as isize);
        *s.offset(0 as std::ffi::c_int as isize) = *s.offset(-(1 as std::ffi::c_int) as isize);
        *s.offset(-(1 as std::ffi::c_int) as isize) = tmp;
        s = s.offset(-1);
        (*p).found_state = s;
        if freq > 124 as std::ffi::c_int as std::ffi::c_uint {
            rescale(p);
        }
    }
    next_context(p);
}

pub unsafe fn update1_0<RC>(p: *mut PPMd7<RC>) {
    let s: *mut State = (*p).found_state;
    let mc: *mut Context = (*p).min_context;
    let mut freq: std::ffi::c_uint = (*s).freq as std::ffi::c_uint;
    let summFreq: std::ffi::c_uint = (*mc).union2.summ_freq as std::ffi::c_uint;
    (*p).prev_success = ((2 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(freq) > summFreq)
        as std::ffi::c_int as std::ffi::c_uint;
    (*p).run_length += (*p).prev_success as i32;
    (*mc).union2.summ_freq = summFreq.wrapping_add(4 as std::ffi::c_int as std::ffi::c_uint) as u16;
    freq = freq.wrapping_add(4 as std::ffi::c_int as std::ffi::c_uint);
    (*s).freq = freq as u8;
    if freq > 124 as std::ffi::c_int as std::ffi::c_uint {
        rescale(p);
    }
    next_context(p);
}

pub unsafe fn update2<RC>(p: *mut PPMd7<RC>) {
    let s: *mut State = (*p).found_state;
    let mut freq: std::ffi::c_uint = (*s).freq as std::ffi::c_uint;
    freq = freq.wrapping_add(4 as std::ffi::c_int as std::ffi::c_uint);
    (*p).run_length = (*p).init_rl;
    (*(*p).min_context).union2.summ_freq =
        ((*(*p).min_context).union2.summ_freq as std::ffi::c_int + 4 as std::ffi::c_int) as u16;
    (*s).freq = freq as u8;
    if freq > 124 as std::ffi::c_int as std::ffi::c_uint {
        rescale(p);
    }
    update_model(p);
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

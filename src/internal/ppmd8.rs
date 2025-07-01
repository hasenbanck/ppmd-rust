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
    min_context: *mut Context,
    max_context: *mut Context,
    found_state: *mut State,
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
    base: *mut u8,
    lo_unit: *mut u8,
    hi_unit: *mut u8,
    text: *mut u8,
    units_start: *mut u8,
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
            min_context: null_mut(),
            max_context: null_mut(),
            found_state: null_mut(),
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
            base: memory_ptr.as_ptr(),
            lo_unit: null_mut(),
            hi_unit: null_mut(),
            text: null_mut(),
            units_start: null_mut(),
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

        unsafe { restart_model(&mut ppmd) };

        Ok(ppmd)
    }
}

unsafe fn insert_node<RC>(p: *mut PPMd8<RC>, node: *mut std::ffi::c_void, indx: std::ffi::c_uint) {
    (*(node as *mut Node)).stamp = 0xFFFFFFFF as std::ffi::c_uint;
    (*(node as *mut Node)).next = (*p).free_list[indx as usize];
    (*(node as *mut Node)).nu = (*p).index2units[indx as usize] as std::ffi::c_uint;
    (*p).free_list[indx as usize] =
        (node as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32;
    (*p).stamps[indx as usize] = ((*p).stamps[indx as usize]).wrapping_add(1);
    (*p).stamps[indx as usize];
}

unsafe fn remove_node<RC>(p: *mut PPMd8<RC>, indx: std::ffi::c_uint) -> *mut std::ffi::c_void {
    let node: *mut Node = ((*p).base).offset((*p).free_list[indx as usize] as isize)
        as *mut std::ffi::c_void as *mut Node;
    (*p).free_list[indx as usize] = (*node).next;
    (*p).stamps[indx as usize] = ((*p).stamps[indx as usize]).wrapping_sub(1);
    (*p).stamps[indx as usize];
    return node as *mut std::ffi::c_void;
}

unsafe fn split_block<RC>(
    p: *mut PPMd8<RC>,
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

unsafe fn glue_free_blocks<RC>(p: *mut PPMd8<RC>) {
    let mut n: u32 = 0;
    (*p).glue_count = ((1 as std::ffi::c_int) << 13 as std::ffi::c_int) as u32;
    (*p).stamps = [0; 38];

    if (*p).lo_unit != (*p).hi_unit {
        (*((*p).lo_unit as *mut std::ffi::c_void as *mut Node)).stamp = 0 as std::ffi::c_int as u32;
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
        let mut next: u32 = (*p).free_list[i as usize];
        (*p).free_list[i as usize] = 0 as std::ffi::c_int as u32;
        while next != 0 as std::ffi::c_int as u32 {
            let node: *mut Node =
                ((*p).base).offset(next as isize) as *mut std::ffi::c_void as *mut Node;
            let mut nu: u32 = (*node).nu;
            *prev = next;
            next = (*node).next;
            if nu != 0 as std::ffi::c_int as u32 {
                let mut node2: *mut Node = 0 as *mut Node;
                prev = &mut (*node).next;
                loop {
                    node2 = node.offset(nu as isize);
                    if !((*node2).stamp == 0xFFFFFFFF as std::ffi::c_uint) {
                        break;
                    }
                    nu = nu.wrapping_add((*node2).nu);
                    (*node2).nu = 0 as std::ffi::c_int as u32;
                    (*node).nu = nu;
                }
            }
        }
        i = i.wrapping_add(1);
        i;
    }
    *prev = 0 as std::ffi::c_int as u32;
    while n != 0 as std::ffi::c_int as u32 {
        let mut node_0: *mut Node =
            ((*p).base).offset(n as isize) as *mut std::ffi::c_void as *mut Node;
        let mut nu_0: u32 = (*node_0).nu;
        let mut i_0: std::ffi::c_uint = 0;
        n = (*node_0).next;
        if nu_0 == 0 as std::ffi::c_int as u32 {
            continue;
        }
        while nu_0 > 128 as std::ffi::c_int as u32 {
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
            nu_0 = nu_0.wrapping_sub(128 as std::ffi::c_int as u32);
            node_0 = node_0.offset(128 as std::ffi::c_int as isize);
        }
        i_0 = (*p).units2index[(nu_0 as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
            as std::ffi::c_uint;
        if (*p).index2units[i_0 as usize] as std::ffi::c_uint != nu_0 {
            i_0 = i_0.wrapping_sub(1);
            let k: std::ffi::c_uint = (*p).index2units[i_0 as usize] as std::ffi::c_uint;
            insert_node(
                p,
                node_0.offset(k as isize) as *mut std::ffi::c_void,
                nu_0.wrapping_sub(k)
                    .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint),
            );
        }
        insert_node(p, node_0 as *mut std::ffi::c_void, i_0);
    }
}

#[inline(never)]
unsafe fn alloc_units_rare<RC>(
    p: *mut PPMd8<RC>,
    index: std::ffi::c_uint,
) -> *mut std::ffi::c_void {
    let mut i: std::ffi::c_uint = 0;
    if (*p).glue_count == 0 as std::ffi::c_int as u32 {
        glue_free_blocks(p);
        if (*p).free_list[index as usize] != 0 as std::ffi::c_int as u32 {
            return remove_node(p, index);
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
            let numBytes: u32 =
                (*p).index2units[index as usize] as std::ffi::c_uint * 12 as std::ffi::c_int as u32;
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
    split_block(p, block, i, index);
    return block;
}

unsafe fn alloc_units<RC>(p: *mut PPMd8<RC>, indx: std::ffi::c_uint) -> *mut std::ffi::c_void {
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

unsafe fn shrink_units<RC>(
    p: *mut PPMd8<RC>,
    oldPtr: *mut std::ffi::c_void,
    oldNU: std::ffi::c_uint,
    newNU: std::ffi::c_uint,
) -> *mut std::ffi::c_void {
    let i0: std::ffi::c_uint = (*p).units2index
        [(oldNU as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
        as std::ffi::c_uint;
    let i1: std::ffi::c_uint = (*p).units2index
        [(newNU as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
        as std::ffi::c_uint;
    if i0 == i1 {
        return oldPtr;
    }
    if (*p).free_list[i1 as usize] != 0 as std::ffi::c_int as u32 {
        let ptr: *mut std::ffi::c_void = remove_node(p, i1);
        let mut d: *mut u32 = ptr as *mut u32;
        let mut z: *const u32 = oldPtr as *const u32;
        let mut n: std::ffi::c_uint = newNU;
        loop {
            *d.offset(0 as std::ffi::c_int as isize) = *z.offset(0 as std::ffi::c_int as isize);
            *d.offset(1 as std::ffi::c_int as isize) = *z.offset(1 as std::ffi::c_int as isize);
            *d.offset(2 as std::ffi::c_int as isize) = *z.offset(2 as std::ffi::c_int as isize);
            z = z.offset(3 as std::ffi::c_int as isize);
            d = d.offset(3 as std::ffi::c_int as isize);
            n = n.wrapping_sub(1);
            if !(n != 0) {
                break;
            }
        }
        insert_node(p, oldPtr, i0);
        return ptr;
    }
    split_block(p, oldPtr, i0, i1);
    return oldPtr;
}

unsafe fn free_units<RC>(p: *mut PPMd8<RC>, ptr: *mut std::ffi::c_void, nu: std::ffi::c_uint) {
    insert_node(
        p,
        ptr,
        (*p).units2index[(nu as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
            as std::ffi::c_uint,
    );
}

unsafe fn special_free_unit<RC>(p: *mut PPMd8<RC>, ptr: *mut std::ffi::c_void) {
    if ptr as *mut u8 != (*p).units_start {
        insert_node(p, ptr, 0 as std::ffi::c_int as std::ffi::c_uint);
    } else {
        (*p).units_start = ((*p).units_start).offset(12 as std::ffi::c_int as isize);
    };
}

unsafe fn expand_text_area<RC>(p: *mut PPMd8<RC>) {
    let mut count: [u32; 38] = [0; 38];
    let mut i: std::ffi::c_uint = 0;
    if (*p).lo_unit != (*p).hi_unit {
        (*((*p).lo_unit as *mut std::ffi::c_void as *mut Node)).stamp = 0 as std::ffi::c_int as u32;
    }
    let mut node: *mut Node = (*p).units_start as *mut std::ffi::c_void as *mut Node;
    while (*node).stamp == 0xFFFFFFFF as std::ffi::c_uint {
        let nu: u32 = (*node).nu;
        (*node).stamp = 0 as std::ffi::c_int as u32;
        count[(*p).units2index[(nu as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
            as usize] = (count[(*p).units2index
            [(nu as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
            as usize])
            .wrapping_add(1);
        count[(*p).units2index[(nu as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
            as usize];
        node = node.offset(nu as isize);
    }
    (*p).units_start = node as *mut u8;
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
                &mut *((*p).free_list).as_mut_ptr().offset(i as isize) as *mut u32 as *mut u32;
            let mut n: u32 = *prev;
            (*p).stamps[i as usize] = ((*p).stamps[i as usize]).wrapping_sub(cnt);
            loop {
                let node_0: *mut Node =
                    ((*p).base).offset(n as isize) as *mut std::ffi::c_void as *mut Node;
                n = (*node_0).next;
                if (*node_0).stamp != 0 as std::ffi::c_int as u32 {
                    prev = &mut (*node_0).next;
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
        i;
    }
}

unsafe fn set_successor(p: *mut State, v: u32) {
    (*p).successor_0 = v as u16;
    (*p).successor_1 = (v >> 16 as std::ffi::c_int) as u16;
}

#[inline(never)]
unsafe fn restart_model<RC>(p: *mut PPMd8<RC>) {
    let mut i: std::ffi::c_uint = 0;
    let mut k: std::ffi::c_uint = 0;
    let mut m: std::ffi::c_uint = 0;
    (*p).free_list = [0; 38];
    (*p).stamps = [0; 38];
    (*p).text = ((*p).base)
        .offset((*p).align_offset as isize)
        .offset(0 as std::ffi::c_int as isize);
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
    (*mc).flags = 0 as std::ffi::c_int as u8;
    (*mc).num_stats = (256 as std::ffi::c_int - 1 as std::ffi::c_int) as u8;
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
    m = 0 as std::ffi::c_int as std::ffi::c_uint;
    i = m;
    while m < 25 as std::ffi::c_int as std::ffi::c_uint {
        while (*p).ns2index[i as usize] as std::ffi::c_uint == m {
            i = i.wrapping_add(1);
            i;
        }
        k = 0 as std::ffi::c_int as std::ffi::c_uint;
        while k < 8 as std::ffi::c_int as std::ffi::c_uint {
            let mut r: std::ffi::c_uint = 0;
            let dest: *mut u16 = ((*p).bin_summ[m as usize]).as_mut_ptr().offset(k as isize);
            let val: u16 = (((1 as std::ffi::c_int) << 7 as std::ffi::c_int + 7 as std::ffi::c_int)
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
            k;
        }
        m = m.wrapping_add(1);
        m;
    }
    m = 0 as std::ffi::c_int as std::ffi::c_uint;
    i = m;
    while m < 24 as std::ffi::c_int as std::ffi::c_uint {
        let mut summ: std::ffi::c_uint = 0;
        let mut s_0: *mut See = 0 as *mut See;
        while (*p).ns2index[(i as usize).wrapping_add(3 as std::ffi::c_int as usize) as usize]
            as std::ffi::c_uint
            == m.wrapping_add(3 as std::ffi::c_int as std::ffi::c_uint)
        {
            i = i.wrapping_add(1);
            i;
        }
        s_0 = ((*p).see[m as usize]).as_mut_ptr();
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
            k;
            s_0 = s_0.offset(1);
            s_0;
        }
        m = m.wrapping_add(1);
        m;
    }
    (*p).dummy_see.summ = 0 as std::ffi::c_int as u16;
    (*p).dummy_see.shift = 7 as std::ffi::c_int as u8;
    (*p).dummy_see.count = 64 as std::ffi::c_int as u8;
}

unsafe fn refresh<RC>(
    p: *mut PPMd8<RC>,
    ctx: *mut Context,
    oldNU: std::ffi::c_uint,
    mut scale: std::ffi::c_uint,
) {
    let mut i: std::ffi::c_uint = (*ctx).num_stats as std::ffi::c_uint;
    let mut escFreq: std::ffi::c_uint = 0;
    let mut sumFreq: std::ffi::c_uint = 0;
    let mut flags: std::ffi::c_uint = 0;
    let mut s: *mut State = shrink_units(
        p,
        ((*p).base).offset((*ctx).union4.stats as isize) as *mut std::ffi::c_void as *mut State
            as *mut std::ffi::c_void,
        oldNU,
        i.wrapping_add(2 as std::ffi::c_int as std::ffi::c_uint) >> 1 as std::ffi::c_int,
    ) as *mut State;
    (*ctx).union4.stats = (s as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32;
    scale |= ((*ctx).union2.summ_freq as u32
        >= (1 as std::ffi::c_int as u32) << 15 as std::ffi::c_int) as std::ffi::c_int
        as std::ffi::c_uint;
    flags =
        ((*s).symbol as std::ffi::c_uint).wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint);
    let mut freq: std::ffi::c_uint = (*s).freq as std::ffi::c_uint;
    escFreq = ((*ctx).union2.summ_freq as std::ffi::c_uint).wrapping_sub(freq);
    freq = freq.wrapping_add(scale) >> scale;
    sumFreq = freq;
    (*s).freq = freq as u8;
    loop {
        s = s.offset(1);
        let mut freq_0: std::ffi::c_uint = (*s).freq as std::ffi::c_uint;
        escFreq = escFreq.wrapping_sub(freq_0);
        freq_0 = freq_0.wrapping_add(scale) >> scale;
        sumFreq = sumFreq.wrapping_add(freq_0);
        (*s).freq = freq_0 as u8;
        flags |= ((*s).symbol as std::ffi::c_uint)
            .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint);
        i = i.wrapping_sub(1);
        if !(i != 0) {
            break;
        }
    }
    (*ctx).union2.summ_freq = sumFreq.wrapping_add(escFreq.wrapping_add(scale) >> scale) as u16;
    (*ctx).flags = ((*ctx).flags as std::ffi::c_uint
        & (((1 as std::ffi::c_int) << 4 as std::ffi::c_int) as std::ffi::c_uint).wrapping_add(
            (((1 as std::ffi::c_int) << 2 as std::ffi::c_int) as std::ffi::c_uint)
                .wrapping_mul(scale),
        ))
    .wrapping_add(
        flags >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
            & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint,
    ) as u8;
}

unsafe fn swap_states(t1: *mut State, t2: *mut State) {
    let tmp: State = *t1;
    *t1 = *t2;
    *t2 = tmp;
}

unsafe fn cut_off<RC>(p: *mut PPMd8<RC>, ctx: *mut Context, order: std::ffi::c_uint) -> u32 {
    let mut ns: std::ffi::c_int = (*ctx).num_stats as std::ffi::c_int;
    let mut nu: std::ffi::c_uint = 0;
    let mut stats: *mut State = 0 as *mut State;
    if ns == 0 as std::ffi::c_int {
        let s: *mut State = &mut (*ctx).union2 as *mut Union2 as *mut State;
        let mut successor: u32 =
            (*s).successor_0 as u32 | ((*s).successor_1 as u32) << 16 as std::ffi::c_int;
        if ((*p).base).offset(successor as isize) as *mut std::ffi::c_void as *mut u8
            >= (*p).units_start
        {
            if order < (*p).max_order {
                successor = cut_off(
                    p,
                    ((*p).base).offset(successor as isize) as *mut std::ffi::c_void as *mut Context,
                    order.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint),
                );
            } else {
                successor = 0 as std::ffi::c_int as u32;
            }
            set_successor(s, successor);
            if successor != 0 || order <= 9 as std::ffi::c_int as std::ffi::c_uint {
                return (ctx as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32;
            }
        }
        special_free_unit(p, ctx as *mut std::ffi::c_void);
        return 0 as std::ffi::c_int as u32;
    }
    nu = (ns as std::ffi::c_uint).wrapping_add(2 as std::ffi::c_int as std::ffi::c_uint)
        >> 1 as std::ffi::c_int;
    let indx: std::ffi::c_uint = (*p).units2index
        [(nu as usize).wrapping_sub(1 as std::ffi::c_int as usize) as usize]
        as std::ffi::c_uint;
    stats = ((*p).base).offset((*ctx).union4.stats as isize) as *mut std::ffi::c_void as *mut State;
    if (stats as *mut u8).offset_from((*p).units_start) as std::ffi::c_long as u32
        <= ((1 as std::ffi::c_int) << 14 as std::ffi::c_int) as u32
        && (*ctx).union4.stats <= (*p).free_list[indx as usize]
    {
        let ptr: *mut std::ffi::c_void = remove_node(p, indx);
        (*ctx).union4.stats = (ptr as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32;
        let mut d: *mut u32 = ptr as *mut u32;
        let mut z: *const u32 = stats as *const std::ffi::c_void as *const u32;
        let mut n: std::ffi::c_uint = nu;
        loop {
            *d.offset(0 as std::ffi::c_int as isize) = *z.offset(0 as std::ffi::c_int as isize);
            *d.offset(1 as std::ffi::c_int as isize) = *z.offset(1 as std::ffi::c_int as isize);
            *d.offset(2 as std::ffi::c_int as isize) = *z.offset(2 as std::ffi::c_int as isize);
            z = z.offset(3 as std::ffi::c_int as isize);
            d = d.offset(3 as std::ffi::c_int as isize);
            n = n.wrapping_sub(1);
            if !(n != 0) {
                break;
            }
        }
        if stats as *mut u8 != (*p).units_start {
            insert_node(p, stats as *mut std::ffi::c_void, indx);
        } else {
            (*p).units_start = ((*p).units_start).offset(
                ((*p).index2units[indx as usize] as std::ffi::c_uint * 12 as std::ffi::c_int as u32)
                    as isize,
            );
        }
        stats = ptr as *mut State;
    }
    let mut s_0: *mut State = stats.offset(ns as std::ffi::c_uint as isize);
    loop {
        let successor_0: u32 =
            (*s_0).successor_0 as u32 | ((*s_0).successor_1 as u32) << 16 as std::ffi::c_int;
        if (((*p).base).offset(successor_0 as isize) as *mut std::ffi::c_void as *mut u8)
            < (*p).units_start
        {
            let fresh1 = ns;
            ns = ns - 1;
            let s2: *mut State = stats.offset(fresh1 as std::ffi::c_uint as isize);
            if order != 0 {
                if s_0 != s2 {
                    *s_0 = *s2;
                }
            } else {
                swap_states(s_0, s2);
                set_successor(s2, 0 as std::ffi::c_int as u32);
            }
        } else if order < (*p).max_order {
            set_successor(
                s_0,
                cut_off(
                    p,
                    ((*p).base).offset(successor_0 as isize) as *mut std::ffi::c_void
                        as *mut Context,
                    order.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint),
                ),
            );
        } else {
            set_successor(s_0, 0 as std::ffi::c_int as u32);
        }
        s_0 = s_0.offset(-1);
        if !(s_0 >= stats) {
            break;
        }
    }
    if ns != (*ctx).num_stats as std::ffi::c_int && order != 0 {
        if ns < 0 as std::ffi::c_int {
            free_units(p, stats as *mut std::ffi::c_void, nu);
            special_free_unit(p, ctx as *mut std::ffi::c_void);
            return 0 as std::ffi::c_int as u32;
        }
        (*ctx).num_stats = ns as u8;
        if ns == 0 as std::ffi::c_int {
            let sym: u8 = (*stats).symbol;
            (*ctx).flags = (((*ctx).flags as std::ffi::c_int
                & (1 as std::ffi::c_int) << 4 as std::ffi::c_int)
                as std::ffi::c_uint)
                .wrapping_add(
                    (sym as std::ffi::c_uint)
                        .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
                        >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
                        & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint,
                ) as u8;
            (*ctx).union2.state2.symbol = sym;
            (*ctx).union2.state2.freq = (((*stats).freq as std::ffi::c_uint)
                .wrapping_add(11 as std::ffi::c_int as std::ffi::c_uint)
                >> 3 as std::ffi::c_int) as u8;
            (*ctx).union4.state4.successor_0 = (*stats).successor_0;
            (*ctx).union4.state4.successor_1 = (*stats).successor_1;
            free_units(p, stats as *mut std::ffi::c_void, nu);
        } else {
            refresh(
                p,
                ctx,
                nu,
                ((*ctx).union2.summ_freq as std::ffi::c_uint
                    > (16 as std::ffi::c_int as std::ffi::c_uint)
                        .wrapping_mul(ns as std::ffi::c_uint)) as std::ffi::c_int
                    as std::ffi::c_uint,
            );
        }
    }
    return (ctx as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32;
}

unsafe fn get_used_memory<RC>(p: *const PPMd8<RC>) -> u32 {
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
            ((*p).stamps[i as usize])
                .wrapping_mul((*p).index2units[i as usize] as std::ffi::c_uint),
        ) as u32 as u32;
        i = i.wrapping_add(1);
        i;
    }
    return ((*p).size)
        .wrapping_sub(((*p).hi_unit).offset_from((*p).lo_unit) as std::ffi::c_long as u32)
        .wrapping_sub(((*p).units_start).offset_from((*p).text) as std::ffi::c_long as u32)
        .wrapping_sub(v * 12 as std::ffi::c_int as u32);
}

unsafe fn restore_model<RC>(p: *mut PPMd8<RC>, ctxError: *mut Context) {
    let mut c: *mut Context = 0 as *mut Context;
    let mut s: *mut State = 0 as *mut State;
    (*p).text = ((*p).base)
        .offset((*p).align_offset as isize)
        .offset(0 as std::ffi::c_int as isize);
    c = (*p).max_context;
    while c != ctxError {
        (*c).num_stats = ((*c).num_stats).wrapping_sub(1);
        if (*c).num_stats as std::ffi::c_int == 0 as std::ffi::c_int {
            s = ((*p).base).offset((*c).union4.stats as isize) as *mut std::ffi::c_void
                as *mut State;
            (*c).flags = (((*c).flags as std::ffi::c_int
                & (1 as std::ffi::c_int) << 4 as std::ffi::c_int)
                as std::ffi::c_uint)
                .wrapping_add(
                    ((*s).symbol as std::ffi::c_uint)
                        .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
                        >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
                        & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint,
                ) as u8;
            (*c).union2.state2.symbol = (*s).symbol;
            (*c).union2.state2.freq = (((*s).freq as std::ffi::c_uint)
                .wrapping_add(11 as std::ffi::c_int as std::ffi::c_uint)
                >> 3 as std::ffi::c_int) as u8;
            (*c).union4.state4.successor_0 = (*s).successor_0;
            (*c).union4.state4.successor_1 = (*s).successor_1;
            special_free_unit(p, s as *mut std::ffi::c_void);
        } else {
            refresh(
                p,
                c,
                ((*c).num_stats as std::ffi::c_uint)
                    .wrapping_add(3 as std::ffi::c_int as std::ffi::c_uint)
                    >> 1 as std::ffi::c_int,
                0 as std::ffi::c_int as std::ffi::c_uint,
            );
        }
        c = ((*p).base).offset((*c).suffix as isize) as *mut std::ffi::c_void as *mut Context;
    }
    while c != (*p).min_context {
        if (*c).num_stats as std::ffi::c_int == 0 as std::ffi::c_int {
            (*c).union2.state2.freq = (((*c).union2.state2.freq as std::ffi::c_uint)
                .wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint)
                >> 1 as std::ffi::c_int) as u8;
        } else {
            (*c).union2.summ_freq =
                ((*c).union2.summ_freq as std::ffi::c_int + 4 as std::ffi::c_int) as u16;
            if (*c).union2.summ_freq as std::ffi::c_int
                > 128 as std::ffi::c_int + 4 as std::ffi::c_int * (*c).num_stats as std::ffi::c_int
            {
                refresh(
                    p,
                    c,
                    ((*c).num_stats as std::ffi::c_uint)
                        .wrapping_add(2 as std::ffi::c_int as std::ffi::c_uint)
                        >> 1 as std::ffi::c_int,
                    1 as std::ffi::c_int as std::ffi::c_uint,
                );
            }
        }
        c = ((*p).base).offset((*c).suffix as isize) as *mut std::ffi::c_void as *mut Context;
    }
    if (*p).restore_method == RestoreMethod::Restart
        || get_used_memory(p) < (*p).size >> 1 as std::ffi::c_int
    {
        restart_model(p);
    } else {
        while (*(*p).max_context).suffix != 0 {
            (*p).max_context = ((*p).base).offset((*(*p).max_context).suffix as isize)
                as *mut std::ffi::c_void as *mut Context;
        }
        loop {
            cut_off(
                p,
                (*p).max_context,
                0 as std::ffi::c_int as std::ffi::c_uint,
            );
            expand_text_area(p);
            if !(get_used_memory(p)
                > 3 as std::ffi::c_int as u32 * ((*p).size >> 2 as std::ffi::c_int))
            {
                break;
            }
        }
        (*p).glue_count = 0 as std::ffi::c_int as u32;
        (*p).order_fall = (*p).max_order;
    }
    (*p).min_context = (*p).max_context;
}
#[inline(never)]
unsafe fn create_successors<RC>(
    p: *mut PPMd8<RC>,
    skip: i32,
    mut s1: *mut State,
    mut c: *mut Context,
) -> *mut Context {
    let mut upBranch: u32 = (*(*p).found_state).successor_0 as u32
        | ((*(*p).found_state).successor_1 as u32) << 16 as std::ffi::c_int;
    let mut newSym: u8 = 0;
    let mut newFreq: u8 = 0;
    let mut flags: u8 = 0;
    let mut numPs: std::ffi::c_uint = 0 as std::ffi::c_int as std::ffi::c_uint;
    let mut ps: [*mut State; 17] = [0 as *mut State; 17];
    if skip == 0 {
        let fresh2 = numPs;
        numPs = numPs.wrapping_add(1);
        ps[fresh2 as usize] = (*p).found_state;
    }
    while (*c).suffix != 0 {
        let mut successor: u32 = 0;
        let mut s: *mut State = 0 as *mut State;
        c = ((*p).base).offset((*c).suffix as isize) as *mut std::ffi::c_void as *mut Context;
        if !s1.is_null() {
            s = s1;
            s1 = 0 as *mut State;
        } else if (*c).num_stats as std::ffi::c_int != 0 as std::ffi::c_int {
            let sym: u8 = (*(*p).found_state).symbol;
            s = ((*p).base).offset((*c).union4.stats as isize) as *mut std::ffi::c_void
                as *mut State;
            while (*s).symbol as std::ffi::c_int != sym as std::ffi::c_int {
                s = s.offset(1);
                s;
            }
            if ((*s).freq as std::ffi::c_int) < 124 as std::ffi::c_int - 9 as std::ffi::c_int {
                (*s).freq = ((*s).freq).wrapping_add(1);
                (*s).freq;
                (*c).union2.summ_freq = ((*c).union2.summ_freq).wrapping_add(1);
                (*c).union2.summ_freq;
            }
        } else {
            s = &mut (*c).union2 as *mut Union2 as *mut State;
            (*s).freq = ((*s).freq as std::ffi::c_int
                + (((*(((*p).base).offset((*c).suffix as isize) as *mut std::ffi::c_void
                    as *mut Context))
                    .num_stats
                    == 0) as std::ffi::c_int
                    & (((*s).freq as std::ffi::c_int) < 24 as std::ffi::c_int) as std::ffi::c_int))
                as u8;
        }
        successor = (*s).successor_0 as u32 | ((*s).successor_1 as u32) << 16 as std::ffi::c_int;
        if successor != upBranch {
            c = ((*p).base).offset(successor as isize) as *mut std::ffi::c_void as *mut Context;
            if numPs == 0 as std::ffi::c_int as std::ffi::c_uint {
                return c;
            }
            break;
        } else {
            let fresh3 = numPs;
            numPs = numPs.wrapping_add(1);
            ps[fresh3 as usize] = s;
        }
    }
    newSym = *(((*p).base).offset(upBranch as isize) as *mut std::ffi::c_void as *const u8);
    upBranch = upBranch.wrapping_add(1);
    upBranch;
    flags = (((*(*p).found_state).symbol as std::ffi::c_uint)
        .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
        >> 8 as std::ffi::c_int - 4 as std::ffi::c_int
        & ((1 as std::ffi::c_int) << 4 as std::ffi::c_int) as std::ffi::c_uint)
        .wrapping_add(
            (newSym as std::ffi::c_uint).wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
                >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
                & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint,
        ) as u8;
    if (*c).num_stats as std::ffi::c_int == 0 as std::ffi::c_int {
        newFreq = (*c).union2.state2.freq;
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
                cf.wrapping_add(2 as std::ffi::c_int as u32 * s0)
                    .wrapping_sub(3 as std::ffi::c_int as u32)
                    / s0
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
        (*c1).flags = flags;
        (*c1).num_stats = 0 as std::ffi::c_int as u8;
        (*c1).union2.state2.symbol = newSym;
        (*c1).union2.state2.freq = newFreq;
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
unsafe fn reduce_order<RC>(
    p: *mut PPMd8<RC>,
    mut s1: *mut State,
    mut c: *mut Context,
) -> *mut Context {
    let mut s: *mut State = 0 as *mut State;
    let c1: *mut Context = c;
    let upBranch: u32 = ((*p).text).offset_from((*p).base) as std::ffi::c_long as u32;
    set_successor((*p).found_state, upBranch);
    (*p).order_fall = ((*p).order_fall).wrapping_add(1);
    (*p).order_fall;
    loop {
        if !s1.is_null() {
            c = ((*p).base).offset((*c).suffix as isize) as *mut std::ffi::c_void as *mut Context;
            s = s1;
            s1 = 0 as *mut State;
        } else {
            if (*c).suffix == 0 {
                return c;
            }
            c = ((*p).base).offset((*c).suffix as isize) as *mut std::ffi::c_void as *mut Context;
            if (*c).num_stats != 0 {
                s = ((*p).base).offset((*c).union4.stats as isize) as *mut std::ffi::c_void
                    as *mut State;
                if (*s).symbol as std::ffi::c_int != (*(*p).found_state).symbol as std::ffi::c_int {
                    loop {
                        s = s.offset(1);
                        s;
                        if !((*s).symbol as std::ffi::c_int
                            != (*(*p).found_state).symbol as std::ffi::c_int)
                        {
                            break;
                        }
                    }
                }
                if ((*s).freq as std::ffi::c_int) < 124 as std::ffi::c_int - 9 as std::ffi::c_int {
                    (*s).freq = ((*s).freq as std::ffi::c_int + 2 as std::ffi::c_int) as u8;
                    (*c).union2.summ_freq =
                        ((*c).union2.summ_freq as std::ffi::c_int + 2 as std::ffi::c_int) as u16;
                }
            } else {
                s = &mut (*c).union2 as *mut Union2 as *mut State;
                (*s).freq = ((*s).freq as std::ffi::c_int
                    + (((*s).freq as std::ffi::c_int) < 32 as std::ffi::c_int) as std::ffi::c_int)
                    as u8;
            }
        }
        if (*s).successor_0 as u32 | ((*s).successor_1 as u32) << 16 as std::ffi::c_int != 0 {
            break;
        }
        set_successor(s, upBranch);
        (*p).order_fall = ((*p).order_fall).wrapping_add(1);
        (*p).order_fall;
    }
    if (*s).successor_0 as u32 | ((*s).successor_1 as u32) << 16 as std::ffi::c_int <= upBranch {
        let mut successor: *mut Context = 0 as *mut Context;
        let s2: *mut State = (*p).found_state;
        (*p).found_state = s;
        successor = create_successors(p, 0 as std::ffi::c_int, 0 as *mut State, c);
        if successor.is_null() {
            set_successor(s, 0 as std::ffi::c_int as u32);
        } else {
            set_successor(
                s,
                (successor as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32,
            );
        }
        (*p).found_state = s2;
    }
    let successor_0: u32 =
        (*s).successor_0 as u32 | ((*s).successor_1 as u32) << 16 as std::ffi::c_int;
    if (*p).order_fall == 1 as std::ffi::c_int as std::ffi::c_uint && c1 == (*p).max_context {
        set_successor((*p).found_state, successor_0);
        (*p).text = ((*p).text).offset(-1);
        (*p).text;
    }
    if successor_0 == 0 as std::ffi::c_int as u32 {
        return 0 as *mut Context;
    }
    return ((*p).base).offset(successor_0 as isize) as *mut std::ffi::c_void as *mut Context;
}

#[inline(never)]
pub unsafe fn update_model<RC>(p: *mut PPMd8<RC>) {
    let mut maxSuccessor: u32 = 0;
    let mut minSuccessor: u32 = (*(*p).found_state).successor_0 as u32
        | ((*(*p).found_state).successor_1 as u32) << 16 as std::ffi::c_int;
    let mut c: *mut Context = 0 as *mut Context;
    let mut s0: std::ffi::c_uint = 0;
    let mut ns: std::ffi::c_uint = 0;
    let fFreq: std::ffi::c_uint = (*(*p).found_state).freq as std::ffi::c_uint;
    let mut flag: u8 = 0;
    let fSymbol: u8 = (*(*p).found_state).symbol;
    let mut s: *mut State = 0 as *mut State;
    if ((*(*p).found_state).freq as std::ffi::c_int) < 124 as std::ffi::c_int / 4 as std::ffi::c_int
        && (*(*p).min_context).suffix != 0 as std::ffi::c_int as u32
    {
        c = ((*p).base).offset((*(*p).min_context).suffix as isize) as *mut std::ffi::c_void
            as *mut Context;
        if (*c).num_stats as std::ffi::c_int == 0 as std::ffi::c_int {
            s = &mut (*c).union2 as *mut Union2 as *mut State;
            if ((*s).freq as std::ffi::c_int) < 32 as std::ffi::c_int {
                (*s).freq = ((*s).freq).wrapping_add(1);
                (*s).freq;
            }
        } else {
            let sym: u8 = (*(*p).found_state).symbol;
            s = ((*p).base).offset((*c).union4.stats as isize) as *mut std::ffi::c_void
                as *mut State;
            if (*s).symbol as std::ffi::c_int != sym as std::ffi::c_int {
                loop {
                    s = s.offset(1);
                    s;
                    if !((*s).symbol as std::ffi::c_int != sym as std::ffi::c_int) {
                        break;
                    }
                }
                if (*s.offset(0 as std::ffi::c_int as isize)).freq as std::ffi::c_int
                    >= (*s.offset(-(1 as std::ffi::c_int) as isize)).freq as std::ffi::c_int
                {
                    swap_states(
                        &mut *s.offset(0 as std::ffi::c_int as isize),
                        &mut *s.offset(-(1 as std::ffi::c_int) as isize),
                    );
                    s = s.offset(-1);
                    s;
                }
            }
            if ((*s).freq as std::ffi::c_int) < 124 as std::ffi::c_int - 9 as std::ffi::c_int {
                (*s).freq = ((*s).freq as std::ffi::c_int + 2 as std::ffi::c_int) as u8;
                (*c).union2.summ_freq =
                    ((*c).union2.summ_freq as std::ffi::c_int + 2 as std::ffi::c_int) as u16;
            }
        }
    }
    c = (*p).max_context;
    if (*p).order_fall == 0 as std::ffi::c_int as std::ffi::c_uint && minSuccessor != 0 {
        let cs: *mut Context = create_successors(p, 1 as std::ffi::c_int, s, (*p).min_context);
        if cs.is_null() {
            set_successor((*p).found_state, 0 as std::ffi::c_int as u32);
            restore_model(p, c);
            return;
        }
        set_successor(
            (*p).found_state,
            (cs as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32,
        );
        (*p).max_context = cs;
        (*p).min_context = (*p).max_context;
        return;
    }
    let mut text: *mut u8 = (*p).text;
    let fresh4 = text;
    text = text.offset(1);
    *fresh4 = (*(*p).found_state).symbol;
    (*p).text = text;
    if text >= (*p).units_start {
        restore_model(p, c);
        return;
    }
    maxSuccessor = text.offset_from((*p).base) as std::ffi::c_long as u32;
    if minSuccessor == 0 {
        let cs_0: *mut Context = reduce_order(p, s, (*p).min_context);
        if cs_0.is_null() {
            restore_model(p, c);
            return;
        }
        minSuccessor = (cs_0 as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32;
    } else if (((*p).base).offset(minSuccessor as isize) as *mut std::ffi::c_void as *mut u8)
        < (*p).units_start
    {
        let cs_1: *mut Context = create_successors(p, 0 as std::ffi::c_int, s, (*p).min_context);
        if cs_1.is_null() {
            restore_model(p, c);
            return;
        }
        minSuccessor = (cs_1 as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32;
    }
    (*p).order_fall = ((*p).order_fall).wrapping_sub(1);
    if (*p).order_fall == 0 as std::ffi::c_int as std::ffi::c_uint {
        maxSuccessor = minSuccessor;
        (*p).text = ((*p).text)
            .offset(-(((*p).max_context != (*p).min_context) as std::ffi::c_int as isize));
    }
    flag = ((fSymbol as std::ffi::c_uint).wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
        >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
        & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint) as u8;
    ns = (*(*p).min_context).num_stats as std::ffi::c_uint;
    s0 = ((*(*p).min_context).union2.summ_freq as std::ffi::c_uint)
        .wrapping_sub(ns)
        .wrapping_sub(fFreq);
    while c != (*p).min_context {
        let mut ns1: std::ffi::c_uint = 0;
        let mut sum: u32 = 0;
        ns1 = (*c).num_stats as std::ffi::c_uint;
        if ns1 != 0 as std::ffi::c_int as std::ffi::c_uint {
            if ns1 & 1 as std::ffi::c_int as std::ffi::c_uint
                != 0 as std::ffi::c_int as std::ffi::c_uint
            {
                let oldNU: std::ffi::c_uint = ns1
                    .wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint)
                    >> 1 as std::ffi::c_int;
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
                        restore_model(p, c);
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
                ((3 as std::ffi::c_int as std::ffi::c_uint)
                    .wrapping_mul(ns1)
                    .wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint)
                    < ns) as std::ffi::c_int as std::ffi::c_uint,
            );
        } else {
            let s_0: *mut State =
                alloc_units(p, 0 as std::ffi::c_int as std::ffi::c_uint) as *mut State;
            if s_0.is_null() {
                restore_model(p, c);
                return;
            }
            let mut freq: std::ffi::c_uint = (*c).union2.state2.freq as std::ffi::c_uint;
            (*s_0).symbol = (*c).union2.state2.symbol;
            (*s_0).successor_0 = (*c).union4.state4.successor_0;
            (*s_0).successor_1 = (*c).union4.state4.successor_1;
            (*c).union4.stats = (s_0 as *mut u8).offset_from((*p).base) as std::ffi::c_long as u32;
            if freq
                < (124 as std::ffi::c_int / 4 as std::ffi::c_int - 1 as std::ffi::c_int)
                    as std::ffi::c_uint
            {
                freq <<= 1 as std::ffi::c_int;
            } else {
                freq = (124 as std::ffi::c_int - 4 as std::ffi::c_int) as std::ffi::c_uint;
            }
            (*s_0).freq = freq as u8;
            sum = freq.wrapping_add((*p).init_esc).wrapping_add(
                (ns > 2 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int
                    as std::ffi::c_uint,
            );
        }
        let s_1: *mut State = (((*p).base).offset((*c).union4.stats as isize)
            as *mut std::ffi::c_void as *mut State)
            .offset(ns1 as isize)
            .offset(1 as std::ffi::c_int as isize);
        let mut cf: u32 =
            2 as std::ffi::c_int as u32 * sum.wrapping_add(6 as std::ffi::c_int as u32) * fFreq;
        let sf: u32 = s0.wrapping_add(sum);
        (*s_1).symbol = fSymbol;
        (*c).num_stats = ns1.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint) as u8;
        set_successor(s_1, maxSuccessor);
        (*c).flags = ((*c).flags as std::ffi::c_int | flag as std::ffi::c_int) as u8;
        if cf < 6 as std::ffi::c_int as u32 * sf {
            cf = (1 as std::ffi::c_int as std::ffi::c_uint)
                .wrapping_add((cf > sf) as std::ffi::c_int as std::ffi::c_uint)
                .wrapping_add(
                    (cf >= 4 as std::ffi::c_int as u32 * sf) as std::ffi::c_int as std::ffi::c_uint,
                );
            sum = sum.wrapping_add(4 as std::ffi::c_int as u32);
        } else {
            cf = (4 as std::ffi::c_int as std::ffi::c_uint)
                .wrapping_add(
                    (cf > 9 as std::ffi::c_int as u32 * sf) as std::ffi::c_int as std::ffi::c_uint,
                )
                .wrapping_add(
                    (cf > 12 as std::ffi::c_int as u32 * sf) as std::ffi::c_int as std::ffi::c_uint,
                )
                .wrapping_add(
                    (cf > 15 as std::ffi::c_int as u32 * sf) as std::ffi::c_int as std::ffi::c_uint,
                );
            sum = sum.wrapping_add(cf);
        }
        (*c).union2.summ_freq = sum as u16;
        (*s_1).freq = cf as u8;
        c = ((*p).base).offset((*c).suffix as isize) as *mut std::ffi::c_void as *mut Context;
    }
    (*p).min_context =
        ((*p).base).offset(minSuccessor as isize) as *mut std::ffi::c_void as *mut Context;
    (*p).max_context = (*p).min_context;
}
#[inline(never)]
unsafe fn rescale<RC>(p: *mut PPMd8<RC>) {
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
    i = (*(*p).min_context).num_stats as std::ffi::c_uint;
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
        (*mc).num_stats = numStatsNew as u8;
        n0 =
            numStats.wrapping_add(2 as std::ffi::c_int as std::ffi::c_uint) >> 1 as std::ffi::c_int;
        if numStatsNew == 0 as std::ffi::c_int as std::ffi::c_uint {
            let mut freq_0: std::ffi::c_uint = (2 as std::ffi::c_int as std::ffi::c_uint)
                .wrapping_mul((*stats).freq as std::ffi::c_uint)
                .wrapping_add(escFreq)
                .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint)
                .wrapping_div(escFreq);
            if freq_0 > (124 as std::ffi::c_int / 3 as std::ffi::c_int) as std::ffi::c_uint {
                freq_0 = (124 as std::ffi::c_int / 3 as std::ffi::c_int) as std::ffi::c_uint;
            }
            (*mc).flags = (((*mc).flags as std::ffi::c_int
                & (1 as std::ffi::c_int) << 4 as std::ffi::c_int)
                as std::ffi::c_uint)
                .wrapping_add(
                    ((*stats).symbol as std::ffi::c_uint)
                        .wrapping_add(0xC0 as std::ffi::c_int as std::ffi::c_uint)
                        >> 8 as std::ffi::c_int - 3 as std::ffi::c_int
                        & ((1 as std::ffi::c_int) << 3 as std::ffi::c_int) as std::ffi::c_uint,
                ) as u8;
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
        n1 = numStatsNew.wrapping_add(2 as std::ffi::c_int as std::ffi::c_uint)
            >> 1 as std::ffi::c_int;
        if n0 != n1 {
            (*mc).union4.stats = (shrink_units(p, stats as *mut std::ffi::c_void, n0, n1)
                as *mut u8)
                .offset_from((*p).base) as std::ffi::c_long as u32;
        }
    }
    let mc_0: *mut Context = (*p).min_context;
    (*mc_0).union2.summ_freq = sumFreq
        .wrapping_add(escFreq)
        .wrapping_sub(escFreq >> 1 as std::ffi::c_int) as u16;
    (*mc_0).flags =
        ((*mc_0).flags as std::ffi::c_int | (1 as std::ffi::c_int) << 2 as std::ffi::c_int) as u8;
    (*p).found_state =
        ((*p).base).offset((*mc_0).union4.stats as isize) as *mut std::ffi::c_void as *mut State;
}

pub unsafe fn make_esc_freq<RC>(
    p: *mut PPMd8<RC>,
    numMasked1: std::ffi::c_uint,
    escFreq: *mut u32,
) -> *mut See {
    let mut see: *mut See = 0 as *mut See;
    let mc: *const Context = (*p).min_context;
    let numStats: std::ffi::c_uint = (*mc).num_stats as std::ffi::c_uint;
    if numStats != 0xFF as std::ffi::c_int as std::ffi::c_uint {
        see = ((*p).see[((*p).ns2index
            [(numStats as usize).wrapping_add(2 as std::ffi::c_int as usize) as usize]
            as std::ffi::c_uint as usize)
            .wrapping_sub(3 as std::ffi::c_int as usize) as usize])
            .as_mut_ptr()
            .offset(
                ((*mc).union2.summ_freq as std::ffi::c_uint
                    > (11 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(
                        numStats.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint),
                    )) as std::ffi::c_int as isize,
            )
            .offset(
                (2 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(
                    ((2 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(numStats)
                        < ((*(((*p).base).offset((*mc).suffix as isize) as *mut std::ffi::c_void
                            as *mut Context))
                            .num_stats as std::ffi::c_uint)
                            .wrapping_add(numMasked1)) as std::ffi::c_int
                        as std::ffi::c_uint,
                ) as isize,
            )
            .offset((*mc).flags as std::ffi::c_int as isize);
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
unsafe fn next_context<RC>(p: *mut PPMd8<RC>) {
    let c: *mut Context = ((*p).base).offset(
        ((*(*p).found_state).successor_0 as u32
            | ((*(*p).found_state).successor_1 as u32) << 16 as std::ffi::c_int) as isize,
    ) as *mut std::ffi::c_void as *mut Context;
    if (*p).order_fall == 0 as std::ffi::c_int as std::ffi::c_uint
        && c as *const u8 >= (*p).units_start as *const u8
    {
        (*p).min_context = c;
        (*p).max_context = (*p).min_context;
    } else {
        update_model(p);
    };
}

pub unsafe fn update1<RC>(p: *mut PPMd8<RC>) {
    let mut s: *mut State = (*p).found_state;
    let mut freq: std::ffi::c_uint = (*s).freq as std::ffi::c_uint;
    freq = freq.wrapping_add(4 as std::ffi::c_int as std::ffi::c_uint);
    (*(*p).min_context).union2.summ_freq =
        ((*(*p).min_context).union2.summ_freq as std::ffi::c_int + 4 as std::ffi::c_int) as u16;
    (*s).freq = freq as u8;
    if freq > (*s.offset(-(1 as std::ffi::c_int) as isize)).freq as std::ffi::c_uint {
        swap_states(s, &mut *s.offset(-(1 as std::ffi::c_int) as isize));
        s = s.offset(-1);
        (*p).found_state = s;
        if freq > 124 as std::ffi::c_int as std::ffi::c_uint {
            rescale(p);
        }
    }
    next_context(p);
}

pub unsafe fn update1_0<RC>(p: *mut PPMd8<RC>) {
    let s: *mut State = (*p).found_state;
    let mc: *mut Context = (*p).min_context;
    let mut freq: std::ffi::c_uint = (*s).freq as std::ffi::c_uint;
    let summFreq: std::ffi::c_uint = (*mc).union2.summ_freq as std::ffi::c_uint;
    (*p).prev_success = ((2 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(freq) >= summFreq)
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

pub unsafe fn update2<RC>(p: *mut PPMd8<RC>) {
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

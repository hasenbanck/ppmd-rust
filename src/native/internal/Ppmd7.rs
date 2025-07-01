#![allow(
    dead_code,
    mutable_transmutes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]

use super::*;

pub type size_t = usize;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct CPpmd7_Context_ {
    pub NumStats: UInt16,
    pub Union2: C2RustUnnamed_0,
    pub Union4: C2RustUnnamed,
    pub Suffix: CPpmd7_Context_Ref,
}

pub type CPpmd7_Context_Ref = UInt32;

pub type CPpmd7_Context = CPpmd7_Context_;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct CPpmd7_RangeDec {
    pub Range: UInt32,
    pub Code: UInt32,
    pub Low: UInt32,
    pub Stream: IByteInPtr,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct CPpmd7z_RangeEnc {
    pub Range: UInt32,
    pub Cache: Byte,
    pub Low: UInt64,
    pub CacheSize: UInt64,
    pub Stream: IByteOutPtr,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct CPpmd7 {
    pub MinContext: *mut CPpmd7_Context,
    pub MaxContext: *mut CPpmd7_Context,
    pub FoundState: *mut CPpmd_State,
    pub OrderFall: std::ffi::c_uint,
    pub InitEsc: std::ffi::c_uint,
    pub PrevSuccess: std::ffi::c_uint,
    pub MaxOrder: std::ffi::c_uint,
    pub HiBitsFlag: std::ffi::c_uint,
    pub RunLength: Int32,
    pub InitRL: Int32,
    pub Size: UInt32,
    pub GlueCount: UInt32,
    pub AlignOffset: UInt32,
    pub Base: *mut Byte,
    pub LoUnit: *mut Byte,
    pub HiUnit: *mut Byte,
    pub Text: *mut Byte,
    pub UnitsStart: *mut Byte,
    pub rc: C2RustUnnamed_1,
    pub Indx2Units: [Byte; 40],
    pub Units2Indx: [Byte; 128],
    pub FreeList: [CPpmd_Void_Ref; 38],
    pub NS2BSIndx: [Byte; 256],
    pub NS2Indx: [Byte; 256],
    pub ExpEscape: [Byte; 16],
    pub DummySee: CPpmd_See,
    pub See: [[CPpmd_See; 16]; 25],
    pub BinSumm: [[UInt16; 64]; 128],
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_1 {
    pub dec: CPpmd7_RangeDec,
    pub enc: CPpmd7z_RangeEnc,
}

pub type PPMD7_CTX_PTR = *mut CPpmd7_Context;
pub type CPpmd7_Node = CPpmd7_Node_;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct CPpmd7_Node_ {
    pub Stamp: UInt16,
    pub NU: UInt16,
    pub Next: CPpmd7_Node_Ref,
    pub Prev: CPpmd7_Node_Ref,
}

pub type CPpmd7_Node_Ref = UInt32;

#[derive(Copy, Clone)]
#[repr(C)]
pub union CPpmd7_Node_Union {
    pub Node: CPpmd7_Node,
    pub NextRef: CPpmd7_Node_Ref,
}

static PPMD7_kExpEscape: [Byte; 16] = [
    25 as std::ffi::c_int as Byte,
    14 as std::ffi::c_int as Byte,
    9 as std::ffi::c_int as Byte,
    7 as std::ffi::c_int as Byte,
    5 as std::ffi::c_int as Byte,
    5 as std::ffi::c_int as Byte,
    4 as std::ffi::c_int as Byte,
    4 as std::ffi::c_int as Byte,
    4 as std::ffi::c_int as Byte,
    3 as std::ffi::c_int as Byte,
    3 as std::ffi::c_int as Byte,
    3 as std::ffi::c_int as Byte,
    2 as std::ffi::c_int as Byte,
    2 as std::ffi::c_int as Byte,
    2 as std::ffi::c_int as Byte,
    2 as std::ffi::c_int as Byte,
];

static PPMD7_kInitBinEsc: [UInt16; 8] = [
    0x3CDD as std::ffi::c_int as UInt16,
    0x1F3F as std::ffi::c_int as UInt16,
    0x59BF as std::ffi::c_int as UInt16,
    0x48F3 as std::ffi::c_int as UInt16,
    0x64A1 as std::ffi::c_int as UInt16,
    0x5ABC as std::ffi::c_int as UInt16,
    0x6632 as std::ffi::c_int as UInt16,
    0x6051 as std::ffi::c_int as UInt16,
];

pub unsafe fn Ppmd7_Construct(mut p: *mut CPpmd7) {
    let mut i: std::ffi::c_uint = 0;
    let mut k: std::ffi::c_uint = 0;
    let mut m: std::ffi::c_uint = 0;
    (*p).Base = 0 as *mut Byte;
    i = 0 as std::ffi::c_int as std::ffi::c_uint;
    k = 0 as std::ffi::c_int as std::ffi::c_uint;
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
        let mut step: std::ffi::c_uint = if i >= 12 as std::ffi::c_int as std::ffi::c_uint {
            4 as std::ffi::c_int as std::ffi::c_uint
        } else {
            (i >> 2 as std::ffi::c_int).wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint)
        };
        loop {
            let fresh0 = k;
            k = k.wrapping_add(1);
            (*p).Units2Indx[fresh0 as usize] = i as Byte;
            step = step.wrapping_sub(1);
            if !(step != 0) {
                break;
            }
        }
        (*p).Indx2Units[i as usize] = k as Byte;
        i = i.wrapping_add(1);
        i;
    }
    (*p).NS2BSIndx[0 as std::ffi::c_int as usize] =
        ((0 as std::ffi::c_int) << 1 as std::ffi::c_int) as Byte;
    (*p).NS2BSIndx[1 as std::ffi::c_int as usize] =
        ((1 as std::ffi::c_int) << 1 as std::ffi::c_int) as Byte;
    ((*p).NS2BSIndx)
        .as_mut_ptr()
        .offset(2)
        .write_bytes((2 << 1) as u8, 9);
    ((*p).NS2BSIndx)
        .as_mut_ptr()
        .offset(11)
        .write_bytes((3 << 1) as u8, 256 - 11);
    i = 0 as std::ffi::c_int as std::ffi::c_uint;
    while i < 3 as std::ffi::c_int as std::ffi::c_uint {
        (*p).NS2Indx[i as usize] = i as Byte;
        i = i.wrapping_add(1);
        i;
    }
    m = i;
    k = 1 as std::ffi::c_int as std::ffi::c_uint;
    while i < 256 as std::ffi::c_int as std::ffi::c_uint {
        (*p).NS2Indx[i as usize] = m as Byte;
        k = k.wrapping_sub(1);
        if k == 0 as std::ffi::c_int as std::ffi::c_uint {
            m = m.wrapping_add(1);
            k = m.wrapping_sub(2 as std::ffi::c_int as std::ffi::c_uint);
        }
        i = i.wrapping_add(1);
        i;
    }
    std::ptr::copy_nonoverlapping(PPMD7_kExpEscape.as_ptr(), ((*p).ExpEscape).as_mut_ptr(), 16);
}

pub unsafe fn Ppmd7_Free(mut p: *mut CPpmd7, mut alloc: ISzAllocPtr) {
    ((*alloc).Free).expect("non-null function pointer")(alloc, (*p).Base as *mut std::ffi::c_void);
    (*p).Size = 0 as std::ffi::c_int as UInt32;
    (*p).Base = 0 as *mut Byte;
}

pub unsafe fn Ppmd7_Alloc(mut p: *mut CPpmd7, mut size: UInt32, mut alloc: ISzAllocPtr) -> BoolInt {
    if ((*p).Base).is_null() || (*p).Size != size {
        Ppmd7_Free(p, alloc);
        (*p).AlignOffset =
            (4 as std::ffi::c_int as UInt32).wrapping_sub(size) & 3 as std::ffi::c_int as UInt32;
        (*p).Base = ((*alloc).Alloc).expect("non-null function pointer")(
            alloc,
            ((*p).AlignOffset).wrapping_add(size) as size_t,
        ) as *mut Byte;
        if ((*p).Base).is_null() {
            return 0 as std::ffi::c_int;
        }
        (*p).Size = size;
    }
    return 1 as std::ffi::c_int;
}
unsafe extern "C" fn Ppmd7_InsertNode(
    mut p: *mut CPpmd7,
    mut node: *mut std::ffi::c_void,
    mut indx: std::ffi::c_uint,
) {
    *(node as *mut CPpmd_Void_Ref) = (*p).FreeList[indx as usize];
    (*p).FreeList[indx as usize] =
        (node as *mut Byte).offset_from((*p).Base) as std::ffi::c_long as UInt32;
}
unsafe extern "C" fn Ppmd7_RemoveNode(
    mut p: *mut CPpmd7,
    mut indx: std::ffi::c_uint,
) -> *mut std::ffi::c_void {
    let mut node: *mut CPpmd_Void_Ref = ((*p).Base).offset((*p).FreeList[indx as usize] as isize)
        as *mut std::ffi::c_void as *mut CPpmd_Void_Ref;
    (*p).FreeList[indx as usize] = *node;
    return node as *mut std::ffi::c_void;
}
unsafe extern "C" fn Ppmd7_SplitBlock(
    mut p: *mut CPpmd7,
    mut ptr: *mut std::ffi::c_void,
    mut oldIndx: std::ffi::c_uint,
    mut newIndx: std::ffi::c_uint,
) {
    let mut i: std::ffi::c_uint = 0;
    let mut nu: std::ffi::c_uint = ((*p).Indx2Units[oldIndx as usize] as std::ffi::c_uint)
        .wrapping_sub((*p).Indx2Units[newIndx as usize] as std::ffi::c_uint);
    ptr = (ptr as *mut Byte).offset(
        ((*p).Indx2Units[newIndx as usize] as std::ffi::c_uint * 12 as std::ffi::c_int as UInt32)
            as isize,
    ) as *mut std::ffi::c_void;
    i = (*p).Units2Indx[(nu as size_t).wrapping_sub(1 as std::ffi::c_int as size_t) as usize]
        as std::ffi::c_uint;
    if (*p).Indx2Units[i as usize] as std::ffi::c_uint != nu {
        i = i.wrapping_sub(1);
        let mut k: std::ffi::c_uint = (*p).Indx2Units[i as usize] as std::ffi::c_uint;
        Ppmd7_InsertNode(
            p,
            (ptr as *mut Byte).offset((k * 12 as std::ffi::c_int as UInt32) as isize)
                as *mut std::ffi::c_void,
            nu.wrapping_sub(k)
                .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint),
        );
    }
    Ppmd7_InsertNode(p, ptr, i);
}
unsafe extern "C" fn Ppmd7_GlueFreeBlocks(mut p: *mut CPpmd7) {
    let mut head: CPpmd7_Node_Ref = 0;
    let mut n: CPpmd7_Node_Ref = 0 as std::ffi::c_int as CPpmd7_Node_Ref;
    (*p).GlueCount = 255 as std::ffi::c_int as UInt32;
    if (*p).LoUnit != (*p).HiUnit {
        (*((*p).LoUnit as *mut std::ffi::c_void as *mut CPpmd7_Node)).Stamp =
            1 as std::ffi::c_int as UInt16;
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
        let nu: UInt16 = (*p).Indx2Units[i as usize] as UInt16;
        let mut next: CPpmd7_Node_Ref = (*p).FreeList[i as usize];
        (*p).FreeList[i as usize] = 0 as std::ffi::c_int as CPpmd_Void_Ref;
        while next != 0 as std::ffi::c_int as CPpmd7_Node_Ref {
            let mut un: *mut CPpmd7_Node_Union =
                ((*p).Base).offset(next as isize) as *mut std::ffi::c_void as *mut CPpmd7_Node
                    as *mut CPpmd7_Node_Union;
            let tmp: CPpmd7_Node_Ref = next;
            next = (*un).NextRef;
            (*un).Node.Stamp = 0 as std::ffi::c_int as UInt16;
            (*un).Node.NU = nu;
            (*un).Node.Next = n;
            n = tmp;
        }
        i = i.wrapping_add(1);
        i;
    }
    head = n;
    let mut prev: *mut CPpmd7_Node_Ref = &mut head;
    while n != 0 {
        let mut node: *mut CPpmd7_Node =
            ((*p).Base).offset(n as isize) as *mut std::ffi::c_void as *mut CPpmd7_Node;
        let mut nu_0: UInt32 = (*node).NU as UInt32;
        n = (*node).Next;
        if nu_0 == 0 as std::ffi::c_int as UInt32 {
            *prev = n;
        } else {
            prev = &mut (*node).Next;
            loop {
                let mut node2: *mut CPpmd7_Node = node.offset(nu_0 as isize);
                nu_0 = nu_0.wrapping_add((*node2).NU as UInt32);
                if (*node2).Stamp as std::ffi::c_int != 0 as std::ffi::c_int
                    || nu_0 >= 0x10000 as std::ffi::c_int as UInt32
                {
                    break;
                }
                (*node).NU = nu_0 as UInt16;
                (*node2).NU = 0 as std::ffi::c_int as UInt16;
            }
        }
    }
    n = head;
    while n != 0 as std::ffi::c_int as CPpmd7_Node_Ref {
        let mut node_0: *mut CPpmd7_Node =
            ((*p).Base).offset(n as isize) as *mut std::ffi::c_void as *mut CPpmd7_Node;
        let mut nu_1: UInt32 = (*node_0).NU as UInt32;
        let mut i_0: std::ffi::c_uint = 0;
        n = (*node_0).Next;
        if nu_1 == 0 as std::ffi::c_int as UInt32 {
            continue;
        }
        while nu_1 > 128 as std::ffi::c_int as UInt32 {
            Ppmd7_InsertNode(
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
            nu_1 = nu_1.wrapping_sub(128 as std::ffi::c_int as UInt32);
            node_0 = node_0.offset(128 as std::ffi::c_int as isize);
        }
        i_0 = (*p).Units2Indx
            [(nu_1 as size_t).wrapping_sub(1 as std::ffi::c_int as size_t) as usize]
            as std::ffi::c_uint;
        if (*p).Indx2Units[i_0 as usize] as std::ffi::c_uint != nu_1 {
            i_0 = i_0.wrapping_sub(1);
            let mut k: std::ffi::c_uint = (*p).Indx2Units[i_0 as usize] as std::ffi::c_uint;
            Ppmd7_InsertNode(
                p,
                node_0.offset(k as isize) as *mut std::ffi::c_void,
                nu_1.wrapping_sub(k)
                    .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint),
            );
        }
        Ppmd7_InsertNode(p, node_0 as *mut std::ffi::c_void, i_0);
    }
}
#[inline(never)]
unsafe extern "C" fn Ppmd7_AllocUnitsRare(
    mut p: *mut CPpmd7,
    mut indx: std::ffi::c_uint,
) -> *mut std::ffi::c_void {
    let mut i: std::ffi::c_uint = 0;
    if (*p).GlueCount == 0 as std::ffi::c_int as UInt32 {
        Ppmd7_GlueFreeBlocks(p);
        if (*p).FreeList[indx as usize] != 0 as std::ffi::c_int as CPpmd_Void_Ref {
            return Ppmd7_RemoveNode(p, indx);
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
            let mut numBytes: UInt32 = (*p).Indx2Units[indx as usize] as std::ffi::c_uint
                * 12 as std::ffi::c_int as UInt32;
            let mut us: *mut Byte = (*p).UnitsStart;
            (*p).GlueCount = ((*p).GlueCount).wrapping_sub(1);
            (*p).GlueCount;
            return (if us.offset_from((*p).Text) as std::ffi::c_long as UInt32 > numBytes {
                (*p).UnitsStart = us.offset(-(numBytes as isize));
                (*p).UnitsStart
            } else {
                0 as *mut Byte
            }) as *mut std::ffi::c_void;
        }
        if !((*p).FreeList[i as usize] == 0 as std::ffi::c_int as CPpmd_Void_Ref) {
            break;
        }
    }
    let mut block: *mut std::ffi::c_void = Ppmd7_RemoveNode(p, i);
    Ppmd7_SplitBlock(p, block, i, indx);
    return block;
}
unsafe extern "C" fn Ppmd7_AllocUnits(
    mut p: *mut CPpmd7,
    mut indx: std::ffi::c_uint,
) -> *mut std::ffi::c_void {
    if (*p).FreeList[indx as usize] != 0 as std::ffi::c_int as CPpmd_Void_Ref {
        return Ppmd7_RemoveNode(p, indx);
    }
    let mut numBytes: UInt32 =
        (*p).Indx2Units[indx as usize] as std::ffi::c_uint * 12 as std::ffi::c_int as UInt32;
    let mut lo: *mut Byte = (*p).LoUnit;
    if ((*p).HiUnit).offset_from(lo) as std::ffi::c_long as UInt32 >= numBytes {
        (*p).LoUnit = lo.offset(numBytes as isize);
        return lo as *mut std::ffi::c_void;
    }
    return Ppmd7_AllocUnitsRare(p, indx);
}
unsafe extern "C" fn SetSuccessor(mut p: *mut CPpmd_State, mut v: CPpmd_Void_Ref) {
    (*p).Successor_0 = v as UInt16;
    (*p).Successor_1 = (v >> 16 as std::ffi::c_int) as UInt16;
}
#[inline(never)]
unsafe extern "C" fn Ppmd7_RestartModel(mut p: *mut CPpmd7) {
    let mut i: std::ffi::c_uint = 0;
    let mut k: std::ffi::c_uint = 0;
    ((*p).FreeList).as_mut_ptr().write_bytes(0, 38);
    (*p).Text = ((*p).Base).offset((*p).AlignOffset as isize);
    (*p).HiUnit = ((*p).Text).offset((*p).Size as isize);
    (*p).UnitsStart = ((*p).HiUnit).offset(
        -(((*p).Size / 8 as std::ffi::c_int as UInt32 / 12 as std::ffi::c_int as UInt32
            * 7 as std::ffi::c_int as UInt32
            * 12 as std::ffi::c_int as UInt32) as isize),
    );
    (*p).LoUnit = (*p).UnitsStart;
    (*p).GlueCount = 0 as std::ffi::c_int as UInt32;
    (*p).OrderFall = (*p).MaxOrder;
    (*p).InitRL = -((if (*p).MaxOrder < 12 as std::ffi::c_int as std::ffi::c_uint {
        (*p).MaxOrder
    } else {
        12 as std::ffi::c_int as std::ffi::c_uint
    }) as Int32)
        - 1 as std::ffi::c_int;
    (*p).RunLength = (*p).InitRL;
    (*p).PrevSuccess = 0 as std::ffi::c_int as std::ffi::c_uint;
    (*p).HiUnit = ((*p).HiUnit).offset(-(12 as std::ffi::c_int as isize));
    let mut mc: *mut CPpmd7_Context = (*p).HiUnit as *mut std::ffi::c_void as PPMD7_CTX_PTR;
    let mut s: *mut CPpmd_State = (*p).LoUnit as *mut CPpmd_State;
    (*p).LoUnit = ((*p).LoUnit).offset(
        ((256 as std::ffi::c_int / 2 as std::ffi::c_int) as UInt32
            * 12 as std::ffi::c_int as UInt32) as isize,
    );
    (*p).MinContext = mc;
    (*p).MaxContext = (*p).MinContext;
    (*p).FoundState = s;
    (*mc).NumStats = 256 as std::ffi::c_int as UInt16;
    (*mc).Union2.SummFreq = (256 as std::ffi::c_int + 1 as std::ffi::c_int) as UInt16;
    (*mc).Union4.Stats = (s as *mut Byte).offset_from((*p).Base) as std::ffi::c_long as UInt32;
    (*mc).Suffix = 0 as std::ffi::c_int as CPpmd7_Context_Ref;
    i = 0 as std::ffi::c_int as std::ffi::c_uint;
    while i < 256 as std::ffi::c_int as std::ffi::c_uint {
        (*s).Symbol = i as Byte;
        (*s).Freq = 1 as std::ffi::c_int as Byte;
        SetSuccessor(s, 0 as std::ffi::c_int as CPpmd_Void_Ref);
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
            let mut dest: *mut UInt16 = ((*p).BinSumm[i as usize]).as_mut_ptr().offset(k as isize);
            let val: UInt16 = (((1 as std::ffi::c_int)
                << 7 as std::ffi::c_int + 7 as std::ffi::c_int)
                as std::ffi::c_uint)
                .wrapping_sub(
                    (PPMD7_kInitBinEsc[k as usize] as std::ffi::c_uint)
                        .wrapping_div(i.wrapping_add(2 as std::ffi::c_int as std::ffi::c_uint)),
                ) as UInt16;
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
        let mut s_0: *mut CPpmd_See = ((*p).See[i as usize]).as_mut_ptr();
        let mut summ: std::ffi::c_uint = (5 as std::ffi::c_int as std::ffi::c_uint)
            .wrapping_mul(i)
            .wrapping_add(10 as std::ffi::c_int as std::ffi::c_uint)
            << 7 as std::ffi::c_int - 4 as std::ffi::c_int;
        k = 0 as std::ffi::c_int as std::ffi::c_uint;
        while k < 16 as std::ffi::c_int as std::ffi::c_uint {
            (*s_0).Summ = summ as UInt16;
            (*s_0).Shift = (7 as std::ffi::c_int - 4 as std::ffi::c_int) as Byte;
            (*s_0).Count = 4 as std::ffi::c_int as Byte;
            k = k.wrapping_add(1);
            k;
            s_0 = s_0.offset(1);
            s_0;
        }
        i = i.wrapping_add(1);
        i;
    }
    (*p).DummySee.Summ = 0 as std::ffi::c_int as UInt16;
    (*p).DummySee.Shift = 7 as std::ffi::c_int as Byte;
    (*p).DummySee.Count = 64 as std::ffi::c_int as Byte;
}

pub unsafe fn Ppmd7_Init(mut p: *mut CPpmd7, mut maxOrder: std::ffi::c_uint) {
    (*p).MaxOrder = maxOrder;
    Ppmd7_RestartModel(p);
}
#[inline(never)]
unsafe extern "C" fn Ppmd7_CreateSuccessors(mut p: *mut CPpmd7) -> PPMD7_CTX_PTR {
    let mut c: PPMD7_CTX_PTR = (*p).MinContext;
    let mut upBranch: CPpmd_Byte_Ref = (*(*p).FoundState).Successor_0 as UInt32
        | ((*(*p).FoundState).Successor_1 as UInt32) << 16 as std::ffi::c_int;
    let mut newSym: Byte = 0;
    let mut newFreq: Byte = 0;
    let mut numPs: std::ffi::c_uint = 0 as std::ffi::c_int as std::ffi::c_uint;
    let mut ps: [*mut CPpmd_State; 64] = [0 as *mut CPpmd_State; 64];
    if (*p).OrderFall != 0 as std::ffi::c_int as std::ffi::c_uint {
        let fresh1 = numPs;
        numPs = numPs.wrapping_add(1);
        ps[fresh1 as usize] = (*p).FoundState;
    }
    while (*c).Suffix != 0 {
        let mut successor: CPpmd_Void_Ref = 0;
        let mut s: *mut CPpmd_State = 0 as *mut CPpmd_State;
        c = ((*p).Base).offset((*c).Suffix as isize) as *mut std::ffi::c_void
            as *mut CPpmd7_Context;
        if (*c).NumStats as std::ffi::c_int != 1 as std::ffi::c_int {
            let mut sym: Byte = (*(*p).FoundState).Symbol;
            s = ((*p).Base).offset((*c).Union4.Stats as isize) as *mut std::ffi::c_void
                as *mut CPpmd_State;
            while (*s).Symbol as std::ffi::c_int != sym as std::ffi::c_int {
                s = s.offset(1);
                s;
            }
        } else {
            s = &mut (*c).Union2 as *mut C2RustUnnamed_0 as *mut CPpmd_State;
        }
        successor =
            (*s).Successor_0 as UInt32 | ((*s).Successor_1 as UInt32) << 16 as std::ffi::c_int;
        if successor != upBranch {
            c = ((*p).Base).offset(successor as isize) as *mut std::ffi::c_void
                as *mut CPpmd7_Context;
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
    newSym = *(((*p).Base).offset(upBranch as isize) as *mut std::ffi::c_void as *const Byte);
    upBranch = upBranch.wrapping_add(1);
    upBranch;
    if (*c).NumStats as std::ffi::c_int == 1 as std::ffi::c_int {
        newFreq = (*(&mut (*c).Union2 as *mut C2RustUnnamed_0 as *mut CPpmd_State)).Freq;
    } else {
        let mut cf: UInt32 = 0;
        let mut s0: UInt32 = 0;
        let mut s_0: *mut CPpmd_State = 0 as *mut CPpmd_State;
        s_0 = ((*p).Base).offset((*c).Union4.Stats as isize) as *mut std::ffi::c_void
            as *mut CPpmd_State;
        while (*s_0).Symbol as std::ffi::c_int != newSym as std::ffi::c_int {
            s_0 = s_0.offset(1);
            s_0;
        }
        cf = ((*s_0).Freq as UInt32).wrapping_sub(1 as std::ffi::c_int as UInt32);
        s0 = ((*c).Union2.SummFreq as UInt32)
            .wrapping_sub((*c).NumStats as UInt32)
            .wrapping_sub(cf);
        newFreq = (1 as std::ffi::c_int as UInt32).wrapping_add(
            (if 2 as std::ffi::c_int as UInt32 * cf <= s0 {
                (5 as std::ffi::c_int as UInt32 * cf > s0) as std::ffi::c_int as UInt32
            } else {
                ((2 as std::ffi::c_int as UInt32 * cf)
                    .wrapping_add(s0)
                    .wrapping_sub(1 as std::ffi::c_int as UInt32)
                    / (2 as std::ffi::c_int as UInt32 * s0))
                    .wrapping_add(1 as std::ffi::c_int as UInt32)
            }),
        ) as Byte;
    }
    loop {
        let mut c1: PPMD7_CTX_PTR = 0 as *mut CPpmd7_Context;
        if (*p).HiUnit != (*p).LoUnit {
            (*p).HiUnit = ((*p).HiUnit).offset(-(12 as std::ffi::c_int as isize));
            c1 = (*p).HiUnit as *mut std::ffi::c_void as PPMD7_CTX_PTR;
        } else if (*p).FreeList[0 as std::ffi::c_int as usize]
            != 0 as std::ffi::c_int as CPpmd_Void_Ref
        {
            c1 = Ppmd7_RemoveNode(p, 0 as std::ffi::c_int as std::ffi::c_uint) as PPMD7_CTX_PTR;
        } else {
            c1 = Ppmd7_AllocUnitsRare(p, 0 as std::ffi::c_int as std::ffi::c_uint) as PPMD7_CTX_PTR;
            if c1.is_null() {
                return 0 as PPMD7_CTX_PTR;
            }
        }
        (*c1).NumStats = 1 as std::ffi::c_int as UInt16;
        (*(&mut (*c1).Union2 as *mut C2RustUnnamed_0 as *mut CPpmd_State)).Symbol = newSym;
        (*(&mut (*c1).Union2 as *mut C2RustUnnamed_0 as *mut CPpmd_State)).Freq = newFreq;
        SetSuccessor(
            &mut (*c1).Union2 as *mut C2RustUnnamed_0 as *mut CPpmd_State,
            upBranch,
        );
        (*c1).Suffix = (c as *mut Byte).offset_from((*p).Base) as std::ffi::c_long as UInt32;
        numPs = numPs.wrapping_sub(1);
        SetSuccessor(
            ps[numPs as usize],
            (c1 as *mut Byte).offset_from((*p).Base) as std::ffi::c_long as UInt32,
        );
        c = c1;
        if !(numPs != 0 as std::ffi::c_int as std::ffi::c_uint) {
            break;
        }
    }
    return c;
}

#[inline(never)]
pub unsafe fn Ppmd7_UpdateModel(mut p: *mut CPpmd7) {
    let mut maxSuccessor: CPpmd_Void_Ref = 0;
    let mut minSuccessor: CPpmd_Void_Ref = 0;
    let mut c: PPMD7_CTX_PTR = 0 as *mut CPpmd7_Context;
    let mut mc: PPMD7_CTX_PTR = 0 as *mut CPpmd7_Context;
    let mut s0: std::ffi::c_uint = 0;
    let mut ns: std::ffi::c_uint = 0;
    if ((*(*p).FoundState).Freq as std::ffi::c_int) < 124 as std::ffi::c_int / 4 as std::ffi::c_int
        && (*(*p).MinContext).Suffix != 0 as std::ffi::c_int as CPpmd7_Context_Ref
    {
        c = ((*p).Base).offset((*(*p).MinContext).Suffix as isize) as *mut std::ffi::c_void
            as *mut CPpmd7_Context;
        if (*c).NumStats as std::ffi::c_int == 1 as std::ffi::c_int {
            let mut s: *mut CPpmd_State =
                &mut (*c).Union2 as *mut C2RustUnnamed_0 as *mut CPpmd_State;
            if ((*s).Freq as std::ffi::c_int) < 32 as std::ffi::c_int {
                (*s).Freq = ((*s).Freq).wrapping_add(1);
                (*s).Freq;
            }
        } else {
            let mut s_0: *mut CPpmd_State = ((*p).Base).offset((*c).Union4.Stats as isize)
                as *mut std::ffi::c_void
                as *mut CPpmd_State;
            let mut sym: Byte = (*(*p).FoundState).Symbol;
            if (*s_0).Symbol as std::ffi::c_int != sym as std::ffi::c_int {
                loop {
                    s_0 = s_0.offset(1);
                    s_0;
                    if !((*s_0).Symbol as std::ffi::c_int != sym as std::ffi::c_int) {
                        break;
                    }
                }
                if (*s_0.offset(0 as std::ffi::c_int as isize)).Freq as std::ffi::c_int
                    >= (*s_0.offset(-(1 as std::ffi::c_int) as isize)).Freq as std::ffi::c_int
                {
                    let mut tmp: CPpmd_State = *s_0.offset(0 as std::ffi::c_int as isize);
                    *s_0.offset(0 as std::ffi::c_int as isize) =
                        *s_0.offset(-(1 as std::ffi::c_int) as isize);
                    *s_0.offset(-(1 as std::ffi::c_int) as isize) = tmp;
                    s_0 = s_0.offset(-1);
                    s_0;
                }
            }
            if ((*s_0).Freq as std::ffi::c_int) < 124 as std::ffi::c_int - 9 as std::ffi::c_int {
                (*s_0).Freq = ((*s_0).Freq as std::ffi::c_int + 2 as std::ffi::c_int) as Byte;
                (*c).Union2.SummFreq =
                    ((*c).Union2.SummFreq as std::ffi::c_int + 2 as std::ffi::c_int) as UInt16;
            }
        }
    }
    if (*p).OrderFall == 0 as std::ffi::c_int as std::ffi::c_uint {
        (*p).MinContext = Ppmd7_CreateSuccessors(p);
        (*p).MaxContext = (*p).MinContext;
        if ((*p).MinContext).is_null() {
            Ppmd7_RestartModel(p);
            return;
        }
        SetSuccessor(
            (*p).FoundState,
            ((*p).MinContext as *mut Byte).offset_from((*p).Base) as std::ffi::c_long as UInt32,
        );
        return;
    }
    let mut text: *mut Byte = (*p).Text;
    let fresh3 = text;
    text = text.offset(1);
    *fresh3 = (*(*p).FoundState).Symbol;
    (*p).Text = text;
    if text >= (*p).UnitsStart {
        Ppmd7_RestartModel(p);
        return;
    }
    maxSuccessor = text.offset_from((*p).Base) as std::ffi::c_long as UInt32;
    minSuccessor = (*(*p).FoundState).Successor_0 as UInt32
        | ((*(*p).FoundState).Successor_1 as UInt32) << 16 as std::ffi::c_int;
    if minSuccessor != 0 {
        if minSuccessor <= maxSuccessor {
            let mut cs: PPMD7_CTX_PTR = Ppmd7_CreateSuccessors(p);
            if cs.is_null() {
                Ppmd7_RestartModel(p);
                return;
            }
            minSuccessor = (cs as *mut Byte).offset_from((*p).Base) as std::ffi::c_long as UInt32;
        }
        (*p).OrderFall = ((*p).OrderFall).wrapping_sub(1);
        if (*p).OrderFall == 0 as std::ffi::c_int as std::ffi::c_uint {
            maxSuccessor = minSuccessor;
            (*p).Text = ((*p).Text)
                .offset(-(((*p).MaxContext != (*p).MinContext) as std::ffi::c_int as isize));
        }
    } else {
        SetSuccessor((*p).FoundState, maxSuccessor);
        minSuccessor =
            ((*p).MinContext as *mut Byte).offset_from((*p).Base) as std::ffi::c_long as UInt32;
    }
    mc = (*p).MinContext;
    c = (*p).MaxContext;
    (*p).MinContext =
        ((*p).Base).offset(minSuccessor as isize) as *mut std::ffi::c_void as *mut CPpmd7_Context;
    (*p).MaxContext = (*p).MinContext;
    if c == mc {
        return;
    }
    ns = (*mc).NumStats as std::ffi::c_uint;
    s0 = ((*mc).Union2.SummFreq as std::ffi::c_uint)
        .wrapping_sub(ns)
        .wrapping_sub(
            ((*(*p).FoundState).Freq as std::ffi::c_uint)
                .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint),
        );
    loop {
        let mut ns1: std::ffi::c_uint = 0;
        let mut sum: UInt32 = 0;
        ns1 = (*c).NumStats as std::ffi::c_uint;
        if ns1 != 1 as std::ffi::c_int as std::ffi::c_uint {
            if ns1 & 1 as std::ffi::c_int as std::ffi::c_uint
                == 0 as std::ffi::c_int as std::ffi::c_uint
            {
                let oldNU: std::ffi::c_uint = ns1 >> 1 as std::ffi::c_int;
                let i: std::ffi::c_uint = (*p).Units2Indx
                    [(oldNU as size_t).wrapping_sub(1 as std::ffi::c_int as size_t) as usize]
                    as std::ffi::c_uint;
                if i != (*p).Units2Indx[(oldNU as size_t)
                    .wrapping_add(1 as std::ffi::c_int as size_t)
                    .wrapping_sub(1 as std::ffi::c_int as size_t)
                    as usize] as std::ffi::c_uint
                {
                    let mut ptr: *mut std::ffi::c_void = Ppmd7_AllocUnits(
                        p,
                        i.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint),
                    );
                    let mut oldPtr: *mut std::ffi::c_void = 0 as *mut std::ffi::c_void;
                    if ptr.is_null() {
                        Ppmd7_RestartModel(p);
                        return;
                    }
                    oldPtr = ((*p).Base).offset((*c).Union4.Stats as isize) as *mut std::ffi::c_void
                        as *mut CPpmd_State as *mut std::ffi::c_void;
                    let mut d: *mut UInt32 = ptr as *mut UInt32;
                    let mut z: *const UInt32 = oldPtr as *const UInt32;
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
                    Ppmd7_InsertNode(p, oldPtr, i);
                    (*c).Union4.Stats =
                        (ptr as *mut Byte).offset_from((*p).Base) as std::ffi::c_long as UInt32;
                }
            }
            sum = (*c).Union2.SummFreq as UInt32;
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
            let mut s_1: *mut CPpmd_State =
                Ppmd7_AllocUnits(p, 0 as std::ffi::c_int as std::ffi::c_uint) as *mut CPpmd_State;
            if s_1.is_null() {
                Ppmd7_RestartModel(p);
                return;
            }
            let mut freq: std::ffi::c_uint = (*c).Union2.State2.Freq as std::ffi::c_uint;
            (*s_1).Symbol = (*c).Union2.State2.Symbol;
            (*s_1).Successor_0 = (*c).Union4.State4.Successor_0;
            (*s_1).Successor_1 = (*c).Union4.State4.Successor_1;
            (*c).Union4.Stats =
                (s_1 as *mut Byte).offset_from((*p).Base) as std::ffi::c_long as UInt32;
            if freq
                < (124 as std::ffi::c_int / 4 as std::ffi::c_int - 1 as std::ffi::c_int)
                    as std::ffi::c_uint
            {
                freq <<= 1 as std::ffi::c_int;
            } else {
                freq = (124 as std::ffi::c_int - 4 as std::ffi::c_int) as std::ffi::c_uint;
            }
            (*s_1).Freq = freq as Byte;
            sum = freq.wrapping_add((*p).InitEsc).wrapping_add(
                (ns > 3 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int
                    as std::ffi::c_uint,
            );
        }
        let mut s_2: *mut CPpmd_State = (((*p).Base).offset((*c).Union4.Stats as isize)
            as *mut std::ffi::c_void as *mut CPpmd_State)
            .offset(ns1 as isize);
        let mut cf: UInt32 = 2 as std::ffi::c_int as UInt32
            * sum.wrapping_add(6 as std::ffi::c_int as UInt32)
            * (*(*p).FoundState).Freq as UInt32;
        let mut sf: UInt32 = s0.wrapping_add(sum);
        (*s_2).Symbol = (*(*p).FoundState).Symbol;
        (*c).NumStats = ns1.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint) as UInt16;
        SetSuccessor(s_2, maxSuccessor);
        if cf < 6 as std::ffi::c_int as UInt32 * sf {
            cf = (1 as std::ffi::c_int as UInt32)
                .wrapping_add((cf > sf) as std::ffi::c_int as UInt32)
                .wrapping_add(
                    (cf >= 4 as std::ffi::c_int as UInt32 * sf) as std::ffi::c_int as UInt32,
                );
            sum = sum.wrapping_add(3 as std::ffi::c_int as UInt32);
        } else {
            cf = (4 as std::ffi::c_int as UInt32)
                .wrapping_add(
                    (cf >= 9 as std::ffi::c_int as UInt32 * sf) as std::ffi::c_int as UInt32,
                )
                .wrapping_add(
                    (cf >= 12 as std::ffi::c_int as UInt32 * sf) as std::ffi::c_int as UInt32,
                )
                .wrapping_add(
                    (cf >= 15 as std::ffi::c_int as UInt32 * sf) as std::ffi::c_int as UInt32,
                );
            sum = sum.wrapping_add(cf);
        }
        (*c).Union2.SummFreq = sum as UInt16;
        (*s_2).Freq = cf as Byte;
        c = ((*p).Base).offset((*c).Suffix as isize) as *mut std::ffi::c_void
            as *mut CPpmd7_Context;
        if !(c != mc) {
            break;
        }
    }
}
#[inline(never)]
unsafe extern "C" fn Ppmd7_Rescale(mut p: *mut CPpmd7) {
    let mut i: std::ffi::c_uint = 0;
    let mut adder: std::ffi::c_uint = 0;
    let mut sumFreq: std::ffi::c_uint = 0;
    let mut escFreq: std::ffi::c_uint = 0;
    let mut stats: *mut CPpmd_State = ((*p).Base).offset((*(*p).MinContext).Union4.Stats as isize)
        as *mut std::ffi::c_void as *mut CPpmd_State;
    let mut s: *mut CPpmd_State = (*p).FoundState;
    if s != stats {
        let mut tmp: CPpmd_State = *s;
        loop {
            *s.offset(0 as std::ffi::c_int as isize) = *s.offset(-(1 as std::ffi::c_int) as isize);
            s = s.offset(-1);
            if !(s != stats) {
                break;
            }
        }
        *s = tmp;
    }
    sumFreq = (*s).Freq as std::ffi::c_uint;
    escFreq = ((*(*p).MinContext).Union2.SummFreq as std::ffi::c_uint).wrapping_sub(sumFreq);
    adder = ((*p).OrderFall != 0 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int
        as std::ffi::c_uint;
    sumFreq = sumFreq
        .wrapping_add(4 as std::ffi::c_int as std::ffi::c_uint)
        .wrapping_add(adder)
        >> 1 as std::ffi::c_int;
    i = ((*(*p).MinContext).NumStats as std::ffi::c_uint)
        .wrapping_sub(1 as std::ffi::c_int as std::ffi::c_uint);
    (*s).Freq = sumFreq as Byte;
    loop {
        s = s.offset(1);
        let mut freq: std::ffi::c_uint = (*s).Freq as std::ffi::c_uint;
        escFreq = escFreq.wrapping_sub(freq);
        freq = freq.wrapping_add(adder) >> 1 as std::ffi::c_int;
        sumFreq = sumFreq.wrapping_add(freq);
        (*s).Freq = freq as Byte;
        if freq > (*s.offset(-(1 as std::ffi::c_int) as isize)).Freq as std::ffi::c_uint {
            let mut tmp_0: CPpmd_State = *s;
            let mut s1: *mut CPpmd_State = s;
            loop {
                *s1.offset(0 as std::ffi::c_int as isize) =
                    *s1.offset(-(1 as std::ffi::c_int) as isize);
                s1 = s1.offset(-1);
                if !(s1 != stats
                    && freq
                        > (*s1.offset(-(1 as std::ffi::c_int) as isize)).Freq as std::ffi::c_uint)
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
    if (*s).Freq as std::ffi::c_int == 0 as std::ffi::c_int {
        let mut mc: *mut CPpmd7_Context = 0 as *mut CPpmd7_Context;
        let mut numStats: std::ffi::c_uint = 0;
        let mut numStatsNew: std::ffi::c_uint = 0;
        let mut n0: std::ffi::c_uint = 0;
        let mut n1: std::ffi::c_uint = 0;
        i = 0 as std::ffi::c_int as std::ffi::c_uint;
        loop {
            i = i.wrapping_add(1);
            i;
            s = s.offset(-1);
            if !((*s).Freq as std::ffi::c_int == 0 as std::ffi::c_int) {
                break;
            }
        }
        escFreq = escFreq.wrapping_add(i);
        mc = (*p).MinContext;
        numStats = (*mc).NumStats as std::ffi::c_uint;
        numStatsNew = numStats.wrapping_sub(i);
        (*mc).NumStats = numStatsNew as UInt16;
        n0 =
            numStats.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint) >> 1 as std::ffi::c_int;
        if numStatsNew == 1 as std::ffi::c_int as std::ffi::c_uint {
            let mut freq_0: std::ffi::c_uint = (*stats).Freq as std::ffi::c_uint;
            loop {
                escFreq >>= 1 as std::ffi::c_int;
                freq_0 = freq_0.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint)
                    >> 1 as std::ffi::c_int;
                if !(escFreq > 1 as std::ffi::c_int as std::ffi::c_uint) {
                    break;
                }
            }
            s = &mut (*mc).Union2 as *mut C2RustUnnamed_0 as *mut CPpmd_State;
            *s = *stats;
            (*s).Freq = freq_0 as Byte;
            (*p).FoundState = s;
            Ppmd7_InsertNode(
                p,
                stats as *mut std::ffi::c_void,
                (*p).Units2Indx
                    [(n0 as size_t).wrapping_sub(1 as std::ffi::c_int as size_t) as usize]
                    as std::ffi::c_uint,
            );
            return;
        }
        n1 = numStatsNew.wrapping_add(1 as std::ffi::c_int as std::ffi::c_uint)
            >> 1 as std::ffi::c_int;
        if n0 != n1 {
            let mut i0: std::ffi::c_uint = (*p).Units2Indx
                [(n0 as size_t).wrapping_sub(1 as std::ffi::c_int as size_t) as usize]
                as std::ffi::c_uint;
            let mut i1: std::ffi::c_uint = (*p).Units2Indx
                [(n1 as size_t).wrapping_sub(1 as std::ffi::c_int as size_t) as usize]
                as std::ffi::c_uint;
            if i0 != i1 {
                if (*p).FreeList[i1 as usize] != 0 as std::ffi::c_int as CPpmd_Void_Ref {
                    let mut ptr: *mut std::ffi::c_void = Ppmd7_RemoveNode(p, i1);
                    (*(*p).MinContext).Union4.Stats =
                        (ptr as *mut Byte).offset_from((*p).Base) as std::ffi::c_long as UInt32;
                    let mut d: *mut UInt32 = ptr as *mut UInt32;
                    let mut z: *const UInt32 = stats as *const std::ffi::c_void as *const UInt32;
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
                    Ppmd7_InsertNode(p, stats as *mut std::ffi::c_void, i0);
                } else {
                    Ppmd7_SplitBlock(p, stats as *mut std::ffi::c_void, i0, i1);
                }
            }
        }
    }
    let mut mc_0: *mut CPpmd7_Context = (*p).MinContext;
    (*mc_0).Union2.SummFreq = sumFreq
        .wrapping_add(escFreq)
        .wrapping_sub(escFreq >> 1 as std::ffi::c_int) as UInt16;
    (*p).FoundState = ((*p).Base).offset((*mc_0).Union4.Stats as isize) as *mut std::ffi::c_void
        as *mut CPpmd_State;
}

pub unsafe fn Ppmd7_MakeEscFreq(
    mut p: *mut CPpmd7,
    mut numMasked: std::ffi::c_uint,
    mut escFreq: *mut UInt32,
) -> *mut CPpmd_See {
    let mut see: *mut CPpmd_See = 0 as *mut CPpmd_See;
    let mut mc: *const CPpmd7_Context = (*p).MinContext;
    let mut numStats: std::ffi::c_uint = (*mc).NumStats as std::ffi::c_uint;
    if numStats != 256 as std::ffi::c_int as std::ffi::c_uint {
        let mut nonMasked: std::ffi::c_uint = numStats.wrapping_sub(numMasked);
        see = ((*p).See[(*p).NS2Indx
            [(nonMasked as size_t).wrapping_sub(1 as std::ffi::c_int as size_t) as usize]
            as std::ffi::c_uint as usize])
            .as_mut_ptr()
            .offset(
                (nonMasked
                    < ((*(((*p).Base).offset((*mc).Suffix as isize) as *mut std::ffi::c_void
                        as *mut CPpmd7_Context))
                        .NumStats as std::ffi::c_uint)
                        .wrapping_sub(numStats)) as std::ffi::c_int as isize,
            )
            .offset((2 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(
                (((*mc).Union2.SummFreq as std::ffi::c_uint)
                    < (11 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(numStats))
                    as std::ffi::c_int as std::ffi::c_uint,
            ) as isize)
            .offset(
                (4 as std::ffi::c_int as std::ffi::c_uint)
                    .wrapping_mul((numMasked > nonMasked) as std::ffi::c_int as std::ffi::c_uint)
                    as isize,
            )
            .offset((*p).HiBitsFlag as isize);
        let summ: std::ffi::c_uint = (*see).Summ as std::ffi::c_uint;
        let r: std::ffi::c_uint = summ >> (*see).Shift as std::ffi::c_int;
        (*see).Summ = summ.wrapping_sub(r) as UInt16;
        *escFreq = r.wrapping_add(
            (r == 0 as std::ffi::c_int as std::ffi::c_uint) as std::ffi::c_int as std::ffi::c_uint,
        );
    } else {
        see = &mut (*p).DummySee;
        *escFreq = 1 as std::ffi::c_int as UInt32;
    }
    return see;
}
unsafe extern "C" fn Ppmd7_NextContext(mut p: *mut CPpmd7) {
    let mut c: PPMD7_CTX_PTR = ((*p).Base).offset(
        ((*(*p).FoundState).Successor_0 as UInt32
            | ((*(*p).FoundState).Successor_1 as UInt32) << 16 as std::ffi::c_int) as isize,
    ) as *mut std::ffi::c_void as *mut CPpmd7_Context;
    if (*p).OrderFall == 0 as std::ffi::c_int as std::ffi::c_uint
        && c as *const Byte > (*p).Text as *const Byte
    {
        (*p).MinContext = c;
        (*p).MaxContext = (*p).MinContext;
    } else {
        Ppmd7_UpdateModel(p);
    };
}

pub unsafe fn Ppmd7_Update1(mut p: *mut CPpmd7) {
    let mut s: *mut CPpmd_State = (*p).FoundState;
    let mut freq: std::ffi::c_uint = (*s).Freq as std::ffi::c_uint;
    freq = freq.wrapping_add(4 as std::ffi::c_int as std::ffi::c_uint);
    (*(*p).MinContext).Union2.SummFreq =
        ((*(*p).MinContext).Union2.SummFreq as std::ffi::c_int + 4 as std::ffi::c_int) as UInt16;
    (*s).Freq = freq as Byte;
    if freq > (*s.offset(-(1 as std::ffi::c_int) as isize)).Freq as std::ffi::c_uint {
        let mut tmp: CPpmd_State = *s.offset(0 as std::ffi::c_int as isize);
        *s.offset(0 as std::ffi::c_int as isize) = *s.offset(-(1 as std::ffi::c_int) as isize);
        *s.offset(-(1 as std::ffi::c_int) as isize) = tmp;
        s = s.offset(-1);
        (*p).FoundState = s;
        if freq > 124 as std::ffi::c_int as std::ffi::c_uint {
            Ppmd7_Rescale(p);
        }
    }
    Ppmd7_NextContext(p);
}

pub unsafe fn Ppmd7_Update1_0(mut p: *mut CPpmd7) {
    let mut s: *mut CPpmd_State = (*p).FoundState;
    let mut mc: *mut CPpmd7_Context = (*p).MinContext;
    let mut freq: std::ffi::c_uint = (*s).Freq as std::ffi::c_uint;
    let summFreq: std::ffi::c_uint = (*mc).Union2.SummFreq as std::ffi::c_uint;
    (*p).PrevSuccess = ((2 as std::ffi::c_int as std::ffi::c_uint).wrapping_mul(freq) > summFreq)
        as std::ffi::c_int as std::ffi::c_uint;
    (*p).RunLength += (*p).PrevSuccess as Int32;
    (*mc).Union2.SummFreq =
        summFreq.wrapping_add(4 as std::ffi::c_int as std::ffi::c_uint) as UInt16;
    freq = freq.wrapping_add(4 as std::ffi::c_int as std::ffi::c_uint);
    (*s).Freq = freq as Byte;
    if freq > 124 as std::ffi::c_int as std::ffi::c_uint {
        Ppmd7_Rescale(p);
    }
    Ppmd7_NextContext(p);
}

pub unsafe fn Ppmd7_Update2(mut p: *mut CPpmd7) {
    let mut s: *mut CPpmd_State = (*p).FoundState;
    let mut freq: std::ffi::c_uint = (*s).Freq as std::ffi::c_uint;
    freq = freq.wrapping_add(4 as std::ffi::c_int as std::ffi::c_uint);
    (*p).RunLength = (*p).InitRL;
    (*(*p).MinContext).Union2.SummFreq =
        ((*(*p).MinContext).Union2.SummFreq as std::ffi::c_int + 4 as std::ffi::c_int) as UInt16;
    (*s).Freq = freq as Byte;
    if freq > 124 as std::ffi::c_int as std::ffi::c_uint {
        Ppmd7_Rescale(p);
    }
    Ppmd7_UpdateModel(p);
}

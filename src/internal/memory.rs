use crate::Error;

/// The size of one allocation unit in bytes.
pub(crate) const UNIT_SIZE: u32 = 12;

/// The size of one state record in bytes. Two states fill one unit.
pub(crate) const STATE_SIZE: u32 = 6;

/// A block of one or more units in the unit area. The offset 0 is the null unit.
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct UnitRef(u32);

impl UnitRef {
    pub(crate) const NULL: Self = Self(0);

    /// Returns the unit at a byte offset in the unit area. The offset 0 gives the null unit.
    #[inline(always)]
    pub(crate) fn at(offset: u32) -> Self {
        Self(offset)
    }

    #[inline(always)]
    pub(crate) fn offset(self) -> u32 {
        self.0
    }

    #[inline(always)]
    pub(crate) fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Returns the unit `nu` units above this unit.
    #[inline(always)]
    pub(crate) fn add_units(self, nu: u32) -> Self {
        Self(self.0 + nu * UNIT_SIZE)
    }

    /// Returns the context record that fills this unit.
    #[inline(always)]
    pub(crate) fn as_ctx(self) -> CtxRef {
        CtxRef(self.0)
    }

    /// Returns the first state of the state array that fills this block.
    #[inline(always)]
    pub(crate) fn as_states(self) -> StateRef {
        StateRef(self.0)
    }
}

/// A context record in the unit area. The offset 0 is the null context.
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct CtxRef(u32);

impl CtxRef {
    pub(crate) const NULL: Self = Self(0);

    /// Returns the context at a byte offset in the unit area. The offset 0 gives the null
    /// context.
    #[inline(always)]
    pub(crate) fn at(offset: u32) -> Self {
        Self(offset)
    }

    #[inline(always)]
    pub(crate) fn offset(self) -> u32 {
        self.0
    }

    #[inline(always)]
    pub(crate) fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Returns the unit that this context fills.
    #[inline(always)]
    pub(crate) fn as_unit(self) -> UnitRef {
        UnitRef(self.0)
    }

    /// Returns the one state of a binary context. It starts at offset 2 of the context in all
    /// model variants.
    #[inline(always)]
    pub(crate) fn one_state(self) -> StateRef {
        StateRef(self.0 + 2)
    }
}

/// A state record. It is either a member of a state array or the one state of a binary context.
/// The offset 0 is the null state.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StateRef(u32);

impl StateRef {
    pub(crate) const NULL: Self = Self(0);

    /// Returns the state at a byte offset. The offset 0 gives the null state.
    #[inline(always)]
    pub(crate) fn at(offset: u32) -> Self {
        Self(offset)
    }

    #[inline(always)]
    pub(crate) fn offset(self) -> u32 {
        self.0
    }

    /// Returns the state `n` states above this state.
    #[inline(always)]
    pub(crate) fn add(self, n: u32) -> Self {
        Self(self.0 + n * STATE_SIZE)
    }

    #[inline(always)]
    pub(crate) fn next(self) -> Self {
        Self(self.0 + STATE_SIZE)
    }

    #[inline(always)]
    pub(crate) fn prev(self) -> Self {
        Self(self.0 - STATE_SIZE)
    }

    /// Returns the block of a state array. The state must be the first state of the array.
    #[inline(always)]
    pub(crate) fn as_unit(self) -> UnitRef {
        UnitRef(self.0)
    }
}

/// The successor of a state. It is null, a raw successor or a real context.
///
/// A raw successor is a position in the text area. It lies below `units_start`. A real
/// context lies in the unit area at or above `units_start`.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Successor(u32);

impl Successor {
    pub(crate) const NULL: Self = Self(0);

    /// Returns the raw successor at a position in the text area.
    #[inline(always)]
    pub(crate) fn text(position: u32) -> Self {
        Self(position)
    }

    #[inline(always)]
    pub(crate) fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Returns the text position of a raw successor.
    #[inline(always)]
    pub(crate) fn text_position(self) -> u32 {
        self.0
    }

    /// Returns true for a real context. A real context lies in the unit area and the text
    /// area lies below it.
    #[inline(always)]
    pub(crate) fn is_real_context(self, units_start: u32) -> bool {
        self.0 >= units_start
    }

    /// Returns the context of a successor. A null successor gives the null context.
    #[inline(always)]
    pub(crate) fn context(self) -> CtxRef {
        CtxRef(self.0)
    }
}

impl From<CtxRef> for Successor {
    #[inline(always)]
    fn from(ctx: CtxRef) -> Self {
        Self(ctx.0)
    }
}

/// The model memory. The text area grows upwards from the start and the unit area fills the
/// end. Every reference into the memory is a byte offset. The offset 0 is the null reference.
///
/// A state record has 6 bytes:
///
/// | Offset | Field             |
/// |--------|-------------------|
/// | 0      | `symbol: u8`      |
/// | 1      | `freq: u8`        |
/// | 2      | `successor: u32`  |
///
/// A context record fills one unit. Its layout depends on the model variant.
pub(crate) struct Memory {
    bytes: Vec<u8>,
}

impl Memory {
    pub(crate) fn new(size: usize) -> Result<Self, Error> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| Error::MemoryAllocation)?;
        bytes.resize(size, 0);
        Ok(Self { bytes })
    }

    #[inline(always)]
    pub(crate) fn u8(&self, offset: u32) -> u8 {
        self.bytes[offset as usize]
    }

    #[inline(always)]
    pub(crate) fn set_u8(&mut self, offset: u32, value: u8) {
        self.bytes[offset as usize] = value;
    }

    #[inline(always)]
    pub(crate) fn u16(&self, offset: u32) -> u16 {
        let offset = offset as usize;
        u16::from_le_bytes(self.bytes[offset..offset + 2].try_into().unwrap())
    }

    #[inline(always)]
    pub(crate) fn set_u16(&mut self, offset: u32, value: u16) {
        let offset = offset as usize;
        self.bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    #[inline(always)]
    pub(crate) fn u32(&self, offset: u32) -> u32 {
        let offset = offset as usize;
        u32::from_le_bytes(self.bytes[offset..offset + 4].try_into().unwrap())
    }

    #[inline(always)]
    pub(crate) fn set_u32(&mut self, offset: u32, value: u32) {
        let offset = offset as usize;
        self.bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// Copies `len` bytes from `src` to `dst`. The ranges may overlap.
    #[inline(always)]
    fn copy(&mut self, src: u32, dst: u32, len: u32) {
        let src = src as usize;
        self.bytes
            .copy_within(src..src + len as usize, dst as usize);
    }

    /// Copies `nu` units from `src` to `dst`.
    #[inline(always)]
    pub(crate) fn copy_units(&mut self, src: UnitRef, dst: UnitRef, nu: u32) {
        self.copy(src.offset(), dst.offset(), nu * UNIT_SIZE);
    }

    /// Returns the byte at a position in the text area.
    #[inline(always)]
    pub(crate) fn text(&self, position: u32) -> u8 {
        self.u8(position)
    }

    /// Sets the byte at a position in the text area.
    #[inline(always)]
    pub(crate) fn set_text(&mut self, position: u32, value: u8) {
        self.set_u8(position, value);
    }

    #[inline(always)]
    pub(crate) fn symbol(&self, state: StateRef) -> u8 {
        self.u8(state.0)
    }

    #[inline(always)]
    pub(crate) fn set_symbol(&mut self, state: StateRef, value: u8) {
        self.set_u8(state.0, value);
    }

    #[inline(always)]
    pub(crate) fn freq(&self, state: StateRef) -> u8 {
        self.u8(state.0 + 1)
    }

    #[inline(always)]
    pub(crate) fn set_freq(&mut self, state: StateRef, value: u8) {
        self.set_u8(state.0 + 1, value);
    }

    #[inline(always)]
    pub(crate) fn successor(&self, state: StateRef) -> Successor {
        Successor(self.u32(state.0 + 2))
    }

    #[inline(always)]
    pub(crate) fn set_successor(&mut self, state: StateRef, value: Successor) {
        self.set_u32(state.0 + 2, value.0);
    }

    #[inline(always)]
    pub(crate) fn state(&self, state: StateRef) -> [u8; STATE_SIZE as usize] {
        let state = state.0 as usize;
        self.bytes[state..state + STATE_SIZE as usize]
            .try_into()
            .unwrap()
    }

    #[inline(always)]
    pub(crate) fn set_state(&mut self, state: StateRef, value: [u8; STATE_SIZE as usize]) {
        let state = state.0 as usize;
        self.bytes[state..state + STATE_SIZE as usize].copy_from_slice(&value);
    }

    /// Copies the state record at `src` to `dst`.
    #[inline(always)]
    pub(crate) fn copy_state(&mut self, src: StateRef, dst: StateRef) {
        self.copy(src.0, dst.0, STATE_SIZE);
    }

    #[inline(always)]
    pub(crate) fn swap_states(&mut self, a: StateRef, b: StateRef) {
        let tmp = self.state(a);
        self.copy_state(b, a);
        self.set_state(b, tmp);
    }
}

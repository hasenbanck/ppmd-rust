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
pub(crate) struct Memory<B = Vec<u8>> {
    bytes: B,
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

    /// Returns a view of the memory that borrows the bytes as a slice. The compiler keeps the
    /// address and the length of the slice in registers across writes into the memory.
    #[inline(always)]
    pub(crate) fn view(&mut self) -> Memory<&mut [u8]> {
        Memory {
            bytes: &mut self.bytes,
        }
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> Memory<B> {
    #[inline(always)]
    fn u8(&self, offset: u32) -> u8 {
        self.bytes.as_ref()[offset as usize]
    }

    #[inline(always)]
    fn set_u8(&mut self, offset: u32, value: u8) {
        self.bytes.as_mut()[offset as usize] = value;
    }

    /// Returns the record of `N` bytes at `offset`. All field accesses of a record check the
    /// same bounds, so the compiler can merge the checks of several fields into one.
    #[inline(always)]
    fn record<const N: usize>(&self, offset: u32) -> &[u8; N] {
        let offset = offset as usize;
        self.bytes.as_ref()[offset..offset + N].try_into().unwrap()
    }

    #[inline(always)]
    fn record_mut<const N: usize>(&mut self, offset: u32) -> &mut [u8; N] {
        let offset = offset as usize;
        (&mut self.bytes.as_mut()[offset..offset + N])
            .try_into()
            .unwrap()
    }

    /// Returns the byte at offset `field` of the unit record at `unit`.
    #[inline(always)]
    pub(crate) fn unit_u8(&self, unit: u32, field: usize) -> u8 {
        self.record::<{ UNIT_SIZE as usize }>(unit)[field]
    }

    #[inline(always)]
    pub(crate) fn set_unit_u8(&mut self, unit: u32, field: usize, value: u8) {
        self.record_mut::<{ UNIT_SIZE as usize }>(unit)[field] = value;
    }

    /// Returns the `u16` at offset `field` of the unit record at `unit`.
    #[inline(always)]
    pub(crate) fn unit_u16(&self, unit: u32, field: usize) -> u16 {
        let record = self.record::<{ UNIT_SIZE as usize }>(unit);
        u16::from_le_bytes(record[field..field + 2].try_into().unwrap())
    }

    #[inline(always)]
    pub(crate) fn set_unit_u16(&mut self, unit: u32, field: usize, value: u16) {
        let record = self.record_mut::<{ UNIT_SIZE as usize }>(unit);
        record[field..field + 2].copy_from_slice(&value.to_le_bytes());
    }

    /// Returns the `u32` at offset `field` of the unit record at `unit`.
    #[inline(always)]
    pub(crate) fn unit_u32(&self, unit: u32, field: usize) -> u32 {
        let record = self.record::<{ UNIT_SIZE as usize }>(unit);
        u32::from_le_bytes(record[field..field + 4].try_into().unwrap())
    }

    #[inline(always)]
    pub(crate) fn set_unit_u32(&mut self, unit: u32, field: usize, value: u32) {
        let record = self.record_mut::<{ UNIT_SIZE as usize }>(unit);
        record[field..field + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// Copies `len` bytes from `src` to `dst`. The ranges may overlap.
    #[inline(always)]
    fn copy(&mut self, src: u32, dst: u32, len: u32) {
        let src = src as usize;
        self.bytes
            .as_mut()
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
        self.state(state)[0]
    }

    #[inline(always)]
    pub(crate) fn set_symbol(&mut self, state: StateRef, value: u8) {
        self.record_mut::<{ STATE_SIZE as usize }>(state.0)[0] = value;
    }

    #[inline(always)]
    pub(crate) fn freq(&self, state: StateRef) -> u8 {
        self.state(state)[1]
    }

    #[inline(always)]
    pub(crate) fn set_freq(&mut self, state: StateRef, value: u8) {
        self.record_mut::<{ STATE_SIZE as usize }>(state.0)[1] = value;
    }

    #[inline(always)]
    pub(crate) fn successor(&self, state: StateRef) -> Successor {
        let state = self.state(state);
        Successor(u32::from_le_bytes([state[2], state[3], state[4], state[5]]))
    }

    #[inline(always)]
    pub(crate) fn set_successor(&mut self, state: StateRef, value: Successor) {
        self.record_mut::<{ STATE_SIZE as usize }>(state.0)[2..]
            .copy_from_slice(&value.0.to_le_bytes());
    }

    #[inline(always)]
    pub(crate) fn state(&self, state: StateRef) -> [u8; STATE_SIZE as usize] {
        *self.record(state.0)
    }

    #[inline(always)]
    pub(crate) fn set_state(&mut self, state: StateRef, value: [u8; STATE_SIZE as usize]) {
        *self.record_mut(state.0) = value;
    }

    /// Returns the records of `n` consecutive states that start at `stats`. One bounds check
    /// covers all of them.
    #[inline(always)]
    pub(crate) fn states(&self, stats: StateRef, n: u32) -> &[u8] {
        let start = stats.0 as usize;
        &self.bytes.as_ref()[start..start + n as usize * STATE_SIZE as usize]
    }

    /// Copies the state record at `src` to `dst`.
    #[inline(always)]
    pub(crate) fn copy_state(&mut self, src: StateRef, dst: StateRef) {
        let state = self.state(src);
        self.set_state(dst, state);
    }

    #[inline(always)]
    pub(crate) fn swap_states(&mut self, a: StateRef, b: StateRef) {
        let tmp = self.state(a);
        self.copy_state(b, a);
        self.set_state(b, tmp);
    }
}

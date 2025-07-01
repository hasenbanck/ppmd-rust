use std::{io::Write, mem::ManuallyDrop};

use crate::{
    byte_writer::ByteWriter,
    internal::ppmd7::{
        alloc, construct, encode_symbol, free, range_encoder_flush, range_encoder_init, Init, PPMd7,
    },
    memory::Memory,
    Error, PPMD7_MAX_MEM_SIZE, PPMD7_MAX_ORDER, PPMD7_MIN_MEM_SIZE, PPMD7_MIN_ORDER, SYM_END,
};

/// An encoder to compress data using PPMd7 (PPMdH) with the 7z range coder.
pub struct Ppmd7Encoder<W: Write> {
    ppmd: PPMd7,
    writer: ByteWriter<W>,
    memory: Memory,
}

impl<W: Write> Ppmd7Encoder<W> {
    /// Creates a new [`Ppmd7Encoder`] which provides a writer over the compressed data.
    ///
    /// The given `order` must be between [`PPMD7_MIN_ORDER`] and [`PPMD7_MAX_ORDER`] (inclusive).
    /// The given `mem_size` must be between [`PPMD7_MIN_MEM_SIZE`] and [`PPMD7_MAX_MEM_SIZE`] (inclusive).
    pub fn new(writer: W, order: u32, mem_size: u32) -> crate::Result<Self> {
        if !(PPMD7_MIN_ORDER..=PPMD7_MAX_ORDER).contains(&order)
            || !(PPMD7_MIN_MEM_SIZE..=PPMD7_MAX_MEM_SIZE).contains(&mem_size)
        {
            return Err(Error::InvalidParameter);
        }

        let mut ppmd = unsafe { std::mem::zeroed::<PPMd7>() };
        unsafe { construct(&mut ppmd) };

        let mut memory = Memory::new(mem_size);

        let success = unsafe { alloc(&mut ppmd, mem_size, memory.allocation()) };

        if success == 0 {
            return Err(Error::MemoryAllocation);
        }

        let mut writer = ByteWriter::new(writer);
        let range_encoder = unsafe { &mut ppmd.rc.enc };
        range_encoder.stream = writer.byte_out_ptr();

        unsafe { range_encoder_init(&mut ppmd) };
        unsafe { Init(&mut ppmd, order) };

        Ok(Self {
            ppmd,
            writer,
            memory,
        })
    }

    /// Returns the inner writer.
    pub fn into_inner(self) -> W {
        let mut manual_drop_self = ManuallyDrop::new(self);
        unsafe {
            free(
                &mut manual_drop_self.ppmd,
                manual_drop_self.memory.allocation(),
            )
        }
        let writer = unsafe { std::ptr::read(&manual_drop_self.writer) };
        writer.inner.writer
    }

    /// Finishes the encoding process.
    ///
    /// Adds an end marker to the data if `with_end_marker` is set to `true`.
    pub fn finish(mut self, with_end_marker: bool) -> Result<W, std::io::Error> {
        unsafe {
            if with_end_marker {
                encode_symbol(&mut self.ppmd, SYM_END);
            }
            self.flush()?;
            Ok(self.into_inner())
        }
    }

    fn inner_flush(&mut self) {
        unsafe { range_encoder_flush(&mut self.ppmd) };
        self.writer.flush();
    }
}

impl<W: Write> Write for Ppmd7Encoder<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        for &byte in buf.iter() {
            unsafe { encode_symbol(&mut self.ppmd, byte as i32) };
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner_flush();
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::io::{Read, Write};

    use crate::{Ppmd7Decoder, Ppmd7Encoder};

    const ORDER: u32 = 8;
    const MEM_SIZE: u32 = 262144;

    #[test]
    fn ppmd7encoder_init_drop() {
        let writer = Vec::new();
        let encoder = Ppmd7Encoder::new(writer, ORDER, MEM_SIZE).unwrap();
        assert!(!encoder.ppmd.base.is_null());
    }

    #[test]
    fn ppmd7encoder_encode_decode() {
        let test_data = "Lorem ipsum dolor sit amet. ";

        let mut writer = Vec::new();
        {
            let mut encoder = Ppmd7Encoder::new(&mut writer, ORDER, MEM_SIZE).unwrap();
            encoder.write_all(test_data.as_bytes()).unwrap();
            encoder.flush().unwrap();
        }

        let mut decoder = Ppmd7Decoder::new(writer.as_slice(), ORDER, MEM_SIZE).unwrap();

        let mut decoded = vec![0; test_data.len()];
        decoder.read_exact(&mut decoded).unwrap();

        assert_eq!(decoded.as_slice(), test_data.as_bytes());

        let decoded_data = String::from_utf8(decoded).unwrap();

        assert_eq!(decoded_data, test_data);
    }
}

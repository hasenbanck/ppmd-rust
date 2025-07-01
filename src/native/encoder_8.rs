use crate::{
    native::{
        internal::ppmd8::{CPpmd8, Ppmd8_Alloc, Ppmd8_Construct, Ppmd8_Free, Ppmd8_Init},
        internal::ppmd8enc::{Ppmd8_EncodeSymbol, Ppmd8_Flush_RangeEnc},
    },
    PPMD8_MAX_MEM_SIZE, PPMD8_MAX_ORDER, PPMD8_MIN_MEM_SIZE, PPMD8_MIN_ORDER, SYM_END,
};

use std::io::Write;

use std::mem::ManuallyDrop;

use super::{byte_writer::ByteWriter, memory::Memory};

use crate::{Error, RestoreMethod};

/// A encoder to compress data using PPMd8 (PPMdI rev.1).
pub struct Ppmd8Encoder<W: Write> {
    ppmd: CPpmd8,
    writer: ByteWriter<W>,
    memory: Memory,
}

impl<W: Write> Ppmd8Encoder<W> {
    /// Creates a new [`Ppmd8Encoder`] which provides a writer over the compressed data.
    ///
    /// The given `order` must be between [`PPMD8_MIN_ORDER`] and [`PPMD8_MAX_ORDER`] (inclusive).
    /// The given `mem_size` must be between [`PPMD8_MIN_MEM_SIZE`] and [`PPMD8_MAX_MEM_SIZE`] (inclusive).
    pub fn new(
        writer: W,
        order: u32,
        mem_size: u32,
        restore_method: RestoreMethod,
    ) -> crate::Result<Self> {
        if !(PPMD8_MIN_ORDER..=PPMD8_MAX_ORDER).contains(&order)
            || !(PPMD8_MIN_MEM_SIZE..=PPMD8_MAX_MEM_SIZE).contains(&mem_size)
        {
            return Err(Error::InvalidParameter);
        }

        let mut ppmd = unsafe { std::mem::zeroed::<CPpmd8>() };
        unsafe { Ppmd8_Construct(&mut ppmd) };

        let mut memory = Memory::new(mem_size);

        let success = unsafe { Ppmd8_Alloc(&mut ppmd, mem_size, memory.allocation()) };

        if success == 0 {
            return Err(Error::MemoryAllocation);
        }

        let mut writer = ByteWriter::new(writer);
        ppmd.Stream.Out = writer.byte_out_ptr();

        // #define Ppmd8_Init_RangeEnc(p) { (p)->Low = 0; (p)->Range = 0xFFFFFFFF; }
        ppmd.Low = 0;
        ppmd.Range = 0xFFFFFFFF;

        unsafe { Ppmd8_Init(&mut ppmd, order, restore_method as _) };

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
            Ppmd8_Free(
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
        if with_end_marker {
            unsafe { Ppmd8_EncodeSymbol(&mut self.ppmd, SYM_END) };
        }
        self.flush()?;
        Ok(self.into_inner())
    }

    fn inner_flush(&mut self) {
        unsafe { Ppmd8_Flush_RangeEnc(&mut self.ppmd) };
        self.writer.flush();
    }
}

impl<W: Write> Write for Ppmd8Encoder<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        buf.iter()
            .for_each(|byte| unsafe { Ppmd8_EncodeSymbol(&mut self.ppmd as *mut _, *byte as _) });

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

    use super::Ppmd8Encoder;
    use crate::{native::Ppmd8Decoder, RestoreMethod};

    const ORDER: u32 = 8;
    const MEM_SIZE: u32 = 262144;
    const RESTORE_METHOD: RestoreMethod = RestoreMethod::Restart;

    #[test]
    fn ppmd8encoder_init_drop() {
        let writer = Vec::new();
        let encoder = Ppmd8Encoder::new(writer, ORDER, MEM_SIZE, RESTORE_METHOD).unwrap();
        assert!(!encoder.ppmd.Base.is_null());
    }

    #[test]
    fn ppmd8encoder_encode_decode() {
        let test_data = "Lorem ipsum dolor sit amet. ";

        let mut writer = Vec::new();
        {
            let mut encoder =
                Ppmd8Encoder::new(&mut writer, ORDER, MEM_SIZE, RESTORE_METHOD).unwrap();
            encoder.write_all(test_data.as_bytes()).unwrap();
            encoder.flush().unwrap();
        }

        let mut decoder =
            Ppmd8Decoder::new(writer.as_slice(), ORDER, MEM_SIZE, RESTORE_METHOD).unwrap();

        let mut decoded = vec![0; test_data.len()];
        decoder.read_exact(&mut decoded).unwrap();

        assert_eq!(decoded.as_slice(), test_data.as_bytes());

        let decoded_data = String::from_utf8(decoded).unwrap();

        assert_eq!(decoded_data, test_data);
    }
}

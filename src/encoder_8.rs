use std::{io::Write, mem::ManuallyDrop};

use crate::{
    byte_writer::ByteWriter,
    internal::ppmd8::{encode_symbol, range_encoder_flush, range_encoder_init, PPMd8, StreamUnion},
    Error, RestoreMethod, PPMD8_MAX_MEM_SIZE, PPMD8_MAX_ORDER, PPMD8_MIN_MEM_SIZE, PPMD8_MIN_ORDER,
    SYM_END,
};

/// A encoder to compress data using PPMd8 (PPMdI rev.1).
pub struct Ppmd8Encoder<W: Write> {
    ppmd: PPMd8,
    writer: ByteWriter<W>,
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

        let mut writer = ByteWriter::new(writer);
        let stream = StreamUnion {
            output: writer.byte_out_ptr(),
        };

        let mut ppmd = PPMd8::construct(stream, order, mem_size, restore_method)?;

        unsafe { range_encoder_init(&mut ppmd) }

        Ok(Self { ppmd, writer })
    }

    /// Returns the inner writer.
    pub fn into_inner(self) -> W {
        let manual_drop_self = ManuallyDrop::new(self);
        let writer = unsafe { std::ptr::read(&manual_drop_self.writer) };
        let ppmd = unsafe { std::ptr::read(&manual_drop_self.ppmd) };
        drop(ppmd);
        writer.inner.writer
    }

    /// Finishes the encoding process.
    ///
    /// Adds an end marker to the data if `with_end_marker` is set to `true`.
    pub fn finish(mut self, with_end_marker: bool) -> Result<W, std::io::Error> {
        if with_end_marker {
            unsafe { encode_symbol(&mut self.ppmd, SYM_END) };
        }
        self.flush()?;
        Ok(self.into_inner())
    }

    fn inner_flush(&mut self) {
        unsafe { range_encoder_flush(&mut self.ppmd) };
        self.writer.flush();
    }
}

impl<W: Write> Write for Ppmd8Encoder<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        for &byte in buf.iter() {
            unsafe { encode_symbol(&mut self.ppmd as *mut _, byte as i32) };
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

    use super::{super::decoder_8::Ppmd8Decoder, Ppmd8Encoder, RestoreMethod};

    const ORDER: u32 = 8;
    const MEM_SIZE: u32 = 262144;
    const RESTORE_METHOD: RestoreMethod = RestoreMethod::Restart;

    #[test]
    fn ppmd8encoder_without_end_marker() {
        let test_data = include_str!("../tests/fixtures/text/apache2.txt");

        let mut writer = Vec::new();
        {
            let mut encoder =
                Ppmd8Encoder::new(&mut writer, ORDER, MEM_SIZE, RESTORE_METHOD).unwrap();
            encoder.write_all(test_data.as_bytes()).unwrap();
            encoder.finish(false).unwrap();
        }

        let mut decoder =
            Ppmd8Decoder::new(writer.as_slice(), ORDER, MEM_SIZE, RESTORE_METHOD).unwrap();

        let mut decoded = vec![0; test_data.len()];
        decoder.read_exact(&mut decoded).unwrap();

        let decoded_data = String::from_utf8(decoded).unwrap();
        assert_eq!(decoded_data, test_data);
    }

    #[test]
    fn ppmd8encoder_with_end_marker() {
        let test_data = include_str!("../tests/fixtures/text/apache2.txt");

        let mut writer = Vec::new();
        {
            let mut encoder =
                Ppmd8Encoder::new(&mut writer, ORDER, MEM_SIZE, RESTORE_METHOD).unwrap();
            encoder.write_all(test_data.as_bytes()).unwrap();
            encoder.finish(true).unwrap();
        }

        let mut decoder =
            Ppmd8Decoder::new(writer.as_slice(), ORDER, MEM_SIZE, RESTORE_METHOD).unwrap();

        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded).unwrap();

        let decoded_data = String::from_utf8(decoded).unwrap();
        assert_eq!(decoded_data, test_data);
    }
}

use std::io::Write;

use crate::{
    Error, PPMD8_MAX_ORDER, PPMD8_MIN_ORDER, RestoreMethod,
    internal::ppmd8::{Ppmd8, RangeEncoder},
};

/// A encoder to encode PPMd8 (PPMdI rev.1) compressed data.
pub struct Ppmd8Encoder<W: Write> {
    ppmd: Ppmd8<RangeEncoder<W>>,
}

impl<W: Write> Ppmd8Encoder<W> {
    /// Creates a new [`Ppmd8Encoder`].
    pub fn new(
        writer: W,
        order: u32,
        mem_size: u32,
        restore_method: RestoreMethod,
    ) -> crate::Result<Self> {
        if !(PPMD8_MIN_ORDER..=PPMD8_MAX_ORDER).contains(&order)
            || restore_method == RestoreMethod::Unsupported
        {
            return Err(Error::InvalidParameter);
        }

        let ppmd = Ppmd8::new_encoder(writer, mem_size, order, restore_method)?;

        Ok(Self { ppmd })
    }

    /// Returns the inner writer.
    pub fn into_inner(self) -> W {
        self.ppmd.into_inner()
    }

    fn inner_flush(&mut self) -> Result<(), std::io::Error> {
        self.ppmd.flush_range_encoder()?;
        Ok(())
    }
}

impl<W: Write> Write for Ppmd8Encoder<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        for &byte in buf {
            self.ppmd.encode_symbol(byte)?;
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner_flush()
    }
}

#[cfg(test)]
mod test {
    use std::io::{Read, Write};

    use super::Ppmd8Encoder;
    use crate::{Ppmd8Decoder, RestoreMethod};

    const ORDER: u32 = 8;
    const MEM_SIZE: u32 = 262144;
    const RESTORE_METHOD: RestoreMethod = RestoreMethod::Restart;

    #[test]
    fn ppmd8encoder_encode_decode() {
        let test_data = include_str!("../tests/fixtures/apache2.txt");

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

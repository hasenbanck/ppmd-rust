use std::io::Read;

use crate::{
    Error, PPMD8_MAX_ORDER, PPMD8_MIN_ORDER, RestoreMethod, SYM_END,
    internal::ppmd8::{Ppmd8, RangeDecoder},
};

/// A decoder to decode PPMd8 (PPMdI rev.1) compressed data.
pub struct Ppmd8Decoder<R: Read> {
    ppmd: Ppmd8<RangeDecoder<R>>,
    finished: bool,
}

impl<R: Read> Ppmd8Decoder<R> {
    /// Creates a new [`Ppmd8Decoder`].
    pub fn new(
        reader: R,
        order: u32,
        mem_size: u32,
        restore_method: RestoreMethod,
    ) -> crate::Result<Self> {
        if !(PPMD8_MIN_ORDER..=PPMD8_MAX_ORDER).contains(&order)
            || restore_method == RestoreMethod::Unsupported
        {
            return Err(Error::InvalidParameter);
        }

        let ppmd = Ppmd8::new_decoder(reader, mem_size, order, restore_method)?;

        Ok(Self {
            ppmd,
            finished: false,
        })
    }

    /// Returns the inner reader.
    pub fn into_inner(self) -> R {
        self.ppmd.into_inner()
    }
}

impl<R: Read> Read for Ppmd8Decoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.finished {
            return Ok(0);
        }

        if buf.is_empty() {
            return Ok(0);
        }

        let mut sym = 0;
        let mut decoded = 0;

        unsafe {
            for byte in buf.iter_mut() {
                sym = self.ppmd.decode_symbol()?;

                if sym < 0 {
                    break;
                }

                *byte = sym as u8;
                decoded += 1;
            }
        }

        let code = self.ppmd.range_decoder_code();

        if sym >= 0 && (!self.finished || decoded != buf.len() || code == 0) {
            return Ok(decoded);
        }

        self.finished = true;

        if sym != SYM_END || code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Error during PPMd decoding",
            ));
        }

        Ok(decoded)
    }
}

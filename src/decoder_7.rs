use std::{io::Read, mem::ManuallyDrop};

use crate::{
    byte_reader::ByteReader,
    internal::ppmd7::{decode_symbol, range_decoder_init, PPMd7, RangeCoder, RangeDecoder},
    Error, PPMD7_MAX_MEM_SIZE, PPMD7_MAX_ORDER, PPMD7_MIN_MEM_SIZE, PPMD7_MIN_ORDER, SYM_END,
};

/// A decoder to decompress data using PPMd7 (PPMdH) with the 7z range coder.
pub struct Ppmd7Decoder<R: Read> {
    ppmd: PPMd7,
    reader: ByteReader<R>,
    finished: bool,
}

impl<R: Read> Ppmd7Decoder<R> {
    /// Creates a new [`Ppmd7Decoder`] which provides a reader over the uncompressed data.
    ///
    /// The given `order` must be between [`PPMD7_MIN_ORDER`] and [`PPMD7_MAX_ORDER`]
    /// The given `mem_size` must be between [`PPMD7_MIN_MEM_SIZE`] and [`PPMD7_MAX_MEM_SIZE`]
    pub fn new(reader: R, order: u32, mem_size: u32) -> crate::Result<Self> {
        if !(PPMD7_MIN_ORDER..=PPMD7_MAX_ORDER).contains(&order)
            || !(PPMD7_MIN_MEM_SIZE..=PPMD7_MAX_MEM_SIZE).contains(&mem_size)
        {
            return Err(Error::InvalidParameter);
        }

        let mut reader = ByteReader::new(reader);
        let mut decoder = RangeDecoder {
            range: 0,
            code: 0,
            low: 0,
            stream: reader.byte_in_ptr(),
        };

        let success = unsafe { range_decoder_init(&mut decoder) };

        if success == 0 {
            return Err(Error::RangeDecoderInitialization);
        }

        let ppmd = PPMd7::new(RangeCoder { dec: decoder }, order, mem_size)?;

        Ok(Self {
            ppmd,
            reader,
            finished: false,
        })
    }

    /// Returns the inner reader.
    pub fn into_inner(self) -> R {
        let manual_drop_self = ManuallyDrop::new(self);
        let reader = unsafe { std::ptr::read(&manual_drop_self.reader) };
        let ppmd = unsafe { std::ptr::read(&manual_drop_self.ppmd) };
        drop(ppmd);
        reader.inner.reader
    }
}

impl<R: Read> Read for Ppmd7Decoder<R> {
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
                sym = decode_symbol(&mut self.ppmd);

                if sym < 0 {
                    break;
                }

                *byte = sym as u8;
                decoded += 1;
            }
        }

        let code = unsafe { self.ppmd.rc.dec.code };

        if sym >= 0 {
            return Ok(decoded);
        }

        self.finished = true;

        if sym != SYM_END || code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Error during PPMd decoding",
            ));
        }

        // END_MARKER detected
        Ok(decoded)
    }
}

#[cfg(test)]
mod test {
    use super::Ppmd7Decoder;

    const ORDER: u32 = 8;
    const MEM_SIZE: u32 = 262144;

    #[test]
    fn ppmd7decoder_init_drop() {
        let reader: &[u8] = &[];
        let decoder = Ppmd7Decoder::new(reader, ORDER, MEM_SIZE).unwrap();
        assert!(!decoder.ppmd.base.is_null());
    }
}

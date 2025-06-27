use std::io::{Read, Write};

use crate::Error;

#[derive(Copy, Clone)]
#[repr(C)]
pub(crate) struct RangeDecoder<R: Read> {
    pub(crate) range: u32,
    pub(crate) code: u32,
    pub(crate) low: u32,
    pub(crate) reader: R,
}

impl<R: Read> RangeDecoder<R> {
    pub(crate) fn new(reader: R) -> crate::Result<Self> {
        let mut encoder = Self {
            range: 0xFFFFFFFF,
            code: 0,
            low: 0,
            reader,
        };

        for _ in 0..4 {
            encoder.code = encoder.code << 8 | encoder.read_byte().map_err(Error::IoError)?;
        }

        if encoder.code == 0xFFFFFFFF {
            return Err(Error::RangeDecoderInitialization);
        }

        Ok(encoder)
    }

    #[inline(always)]
    pub(crate) fn correct_sum_range(&self, sum: u32) -> u32 {
        correct_sum_range(self.range, sum)
    }

    #[inline(always)]
    pub(crate) fn read_byte(&mut self) -> Result<u32, std::io::Error> {
        let mut buffer = [0];
        self.reader.read_exact(&mut buffer)?;
        Ok(buffer[0] as u32)
    }

    #[inline(always)]
    pub(crate) fn decode(&mut self, mut start: u32, size: u32) {
        start *= self.range;
        self.low += start;
        self.code -= start;
        self.range *= size;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub(crate) struct RangeEncoder<W: Write> {
    pub(crate) range: u32,
    pub(crate) low: u32,
    pub(crate) writer: W,
}

impl<W: Write> RangeEncoder<W> {
    pub(crate) fn new(writer: W) -> Self {
        Self {
            range: 0xFFFFFFFF,
            low: 0,
            writer,
        }
    }

    #[inline(always)]
    pub(crate) fn correct_sum_range(&self, sum: u32) -> u32 {
        correct_sum_range(self.range, sum)
    }

    #[inline(always)]
    pub(crate) fn write_byte(&mut self, byte: u8) -> Result<(), std::io::Error> {
        self.writer.write_all(&[byte])
    }

    #[inline(always)]
    pub(crate) fn encode(&mut self, start: u32, size: u32, total: u32) {
        self.range /= total;
        self.low += start * self.range;
        self.range *= size;
    }

    pub(crate) fn flush(&mut self) -> Result<(), std::io::Error> {
        for _ in 0..4 {
            let byte = (self.low >> 24) as u8;
            self.writer.write_all(&[byte])?;
            self.low <<= 8;
        }
        self.writer.flush()?;
        Ok(())
    }
}

// The original PPMdI encoder and decoder probably could work incorrectly in some rare cases,
// where the original PPMdI code can give "Divide by Zero" operation.
// We use the following fix to allow correct working of encoder and decoder in any cases.
// We correct (escape_freq) and (sum), if (sum) is larger than (range).
#[inline(always)]
fn correct_sum_range(range: u32, sum: u32) -> u32 {
    if sum > range { range } else { sum }
}

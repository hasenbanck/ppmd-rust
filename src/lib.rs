//! PPMd compression / decompression. It's a port of the PPMd C-code from 7-Zip to Rust.
//!
//! The following variants are provided:
//!
//! - The PPMd7 (PPMdH) as used by the 7z archive format
//! - The PPMd8 (PPMdI rev.1) as used by the zip archive format
//!
//!
//! ## Acknowledgement
//!
//! This port is based on the 7zip version of PPMd by Igor Pavlov, which in turn was based on the
//! PPMd var.H (2001) / PPMd var.I (2002) code by Dmitry Shkarin. The carryless range coder of
//! PPMd8 was originally written by Dmitry Subbotin (1999).
//!
//! ## License
//!
//! The code in this crate is in the public domain as the original code by their authors.
mod decoder_7;
mod encoder_7;

mod decoder_8;
mod encoder_8;
mod internal;

pub use decoder_7::Ppmd7Decoder;
pub use decoder_8::Ppmd8Decoder;
pub use encoder_7::Ppmd7Encoder;
pub use encoder_8::Ppmd8Encoder;

pub const PPMD7_MIN_ORDER: u32 = 2;

pub const PPMD7_MAX_ORDER: u32 = 64;

pub const PPMD7_MIN_MEM_SIZE: u32 = 2048;

pub const PPMD7_MAX_MEM_SIZE: u32 = 4294967259;

pub const PPMD8_MIN_ORDER: u32 = 2;

pub const PPMD8_MAX_ORDER: u32 = 16;

const SYM_END: i32 = -1;
const SYM_ERROR: i32 = -2;

pub type Result<T> = core::result::Result<T, Error>;

/// The restore method used in PPMd8.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum RestoreMethod {
    Restart = 0 as _,
    CutOff = 1 as _,
    Unsupported = 2 as _,
}

macro_rules! impl_from_int_for_restore_method {
    ($($int_type:ty),+ $(,)?) => {
        $(
            impl From<$int_type> for RestoreMethod {
                fn from(value: $int_type) -> Self {
                    match value {
                        0 => RestoreMethod::Restart,
                        1 => RestoreMethod::CutOff,
                        _ => RestoreMethod::Unsupported,
                    }
                }
            }
        )+
    };
}

impl_from_int_for_restore_method!(u8, u16, u32, u64, u128, usize);
impl_from_int_for_restore_method!(i8, i16, i32, i64, i128, isize);

/// Crate error type.
pub enum Error {
    /// The range decoder could not be properly initialized.
    RangeDecoderInitialization,
    /// Invalid parameters provided.
    InvalidParameter,
    /// General IO error.
    IoError(std::io::Error),
    /// The needed memory could not be allocated.
    MemoryAllocation,
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::RangeDecoderInitialization => {
                write!(f, "Could not initialize the range decoder")
            }
            Error::InvalidParameter => write!(f, "Wrong PPMd parameter"),
            Error::IoError(err) => write!(f, "Io error: {err}"),
            Error::MemoryAllocation => write!(f, "Memory allocation error (out of memory?)"),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}

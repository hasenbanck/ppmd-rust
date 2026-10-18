# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 1.5.2 - 2026-10-18

### Changed

- The implementation is now 100% safe Rust and the crate forbids `unsafe` code.
- Fully binary compatible with 1.5.1. Both versions produce byte-identical compressed data and decode each
  other's data.
- Deprecated the `unstable-tagged-offsets` feature. The bounds checks of the safe implementation replace it.
  It's inert and will be removed in version 2 of the crate.

### Fixed

- Fixed `Write::flush` of the encoders ending the compressed stream, which corrupted all data written after a
  `flush()` call. `flush()` now flushes the inner writer, and `finish()` writes the final bytes of the range coder.

## 1.5.1 - 2026-10-18

### Fixed

- Fixed an infinite loop in the PPMd8 model restoration for a memory size between 2048 and 2063 bytes.
  `PPMD8_MIN_MEM_SIZE` is now 2064. The previous value 2048 was the minimum memory size of PPMd7. 7-Zip
  defines no minimum memory size for PPMd8 and has the same infinite loop for these sizes. The ZIP and
  `.pmd` codecs of 7-Zip always use at least 1 MiB and are unaffected. (#19)

## 1.5.0 - 2026-09-14

### Added

- Added `Ppmd7aDecoder` and `Ppmd7aEncoder` for PPMd7 (PPMdH) with the carryless range coder (PPMd7a), as used
  by the `.pmd` files of the original PPMd program.

## 1.4.1 - 2026-09-01

### Fixed

- Fixed out-of-bounds heap reads and writes in the PPMd8 model restoration path (GHSA-rqc2-j9v2-j22v).

## 1.4.0 - 2026-01-23

### Changed

- Re-licensed this crate under CC0-1.0 and MIT-0.
- There are no functional changes between 1.3 and 1.4.

## 1.3.0 - 2025-11-01

### Added

- Added "get_ref/get_mut" to all decoder and encoder to geta access to the inner reader / writer. By roblabla (#10)

## 1.2.1 - 2025-07-05

### Added

- Add an `unstable-tagged-offsets` unstable feature used to validate the proper function of the implementation. It is
  not activated as a general safety feature, since it would reduce the maximal allowed memory size and in such break
  compatibility with existing data.

## 1.2.0 - 2025-07-03

### Added

- Auto traits Send and Sync are implemented for the decoder and encoder.

## 1.1.2 - 2025-07-03

### Fixed

- No functional changes
- Fixed links in cargo to fix broken link in crates.io

## 1.1.1 - 2025-07-03

### Fixed

- Fixed errors in the ported code that created different PPMd code for both PPMd7 and PPMd7:
  (https://github.com/hasenbanck/ppmd-rust/issues/3)

### Added

- Added bigger test corpus to verify implementation.

## 1.1.0 - 2025-06-28

### Added

- Added a `finish()` method on the encoders, so that finishing the encoding process is more straightforward.
- Allow the encoding of an end marker into the compressed data to properly support cases, where the uncompressed data
  size if not known at decompression time.

## 1.0.0 - 2025-06-27

### Updated

- Ported the C code for PPMd8 to Rust (as used in the ZIP archive format).
- Lowered MSRV to 1.82

## 0.3.0 - 2025-06-22

### Updated

- Ported the C code for PPMd7 to Rust (as used in the 7z archive format).

## 0.2.0 - 2025-02-28

### Added

- Provided a safe Rust abstraction over the 7zip PPMd C library

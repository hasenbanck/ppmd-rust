# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 1.1.0 - 2025-06-28

### Added

- Added a `finish()` method on the encoders, so that finishing the encoding process is more straightforward.
- Allow the encoding of an end marker into the compressed data to properly
  support cases, where the uncompressed data size if not known at decompression time.

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

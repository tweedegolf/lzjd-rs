//! # LZJD
//! Rust implementation of the LZJD algorithm
//! See also: https://github.com/EdwardRaff/jLZJD
//!
//! Any core::hash::BuildHasher is supported, just pass a &BuildHasher to LZDict::from_bytes_stream.
//! For convenience, this crate provides a wrapper around the crc32 hasher which implements BuildHasher.
//!
//! ## Example
//! ```
//! # use lzjd::lz_dict::LZDict;
//! # use crc::crc32::{self, Hasher32};
//! # use std::hash::BuildHasher;
//! # use std::hash::Hasher;
//! # pub struct CRC32Hasher {
//! #   digest: crc::crc32::Digest,
//! # }
//! #
//! # impl CRC32Hasher {
//! #     fn new() -> Self {
//! #       Self {
//! #           digest: crc32::Digest::new(crc::crc32::IEEE),
//! #       }
//! #   }
//! # }
//! # impl Hasher for CRC32Hasher {
//! #     fn write(&mut self, bytes: &[u8]) {
//! #         Hasher32::write(&mut self.digest, bytes);
//! #     }
//! #     fn finish(&self) -> u64 {
//! #         u64::from(self.digest.sum32())
//! #     }
//! # }
//! # #[derive(Clone)]
//! # pub struct CRC32BuildHasher;
//! #
//! # impl BuildHasher for CRC32BuildHasher {
//! #   type Hasher = CRC32Hasher;
//! #   fn build_hasher(&self) -> Self::Hasher {
//! #       CRC32Hasher::new()
//! #    }
//! # }
//! let stream_a = b"bitsandpieces".iter().cloned();
//! let stream_b = b"doctestbits".iter().cloned();
//! let k = 1024;
//!
//! let build_hasher = CRC32BuildHasher;
//!
//! let dict_a = LZDict::from_bytes_stream(stream_a, &build_hasher);
//! let dict_b = LZDict::from_bytes_stream(stream_b, &build_hasher);
//!
//! let lzjd = dict_a.dist(&dict_b);
//!
//! assert_eq!(lzjd, 0.5714285714285714);
//! ```

#[macro_use]
extern crate failure_derive;

pub use crate::lz_dict::LZDict;
use std::io;

/// LZ dictionary implementation
pub mod lz_dict;
/// crc32 wrapper;
pub mod crc32;
/// murmur3 wrapper;
pub mod murmur3;

#[derive(Debug, Fail)]
pub enum LZJDError {
    #[fail(display = "IO error: {}", err)]
    Io {
        #[cause]
        err: io::Error,
    },
    #[fail(display = "Decode error: {}", err)]
    Base64 {
        #[cause]
        err: base64::DecodeError,
    },
    #[fail(display = "Bincode error: {}", err)]
    Bincode {
        #[cause]
        err: bincode::Error,
    },
    #[fail(display = "Error: {}", msg)]
    Msg { msg: String },
}

impl From<base64::DecodeError> for LZJDError {
    fn from(err: base64::DecodeError) -> Self {
        LZJDError::Base64 { err }
    }
}

impl From<bincode::Error> for LZJDError {
    fn from(err: bincode::Error) -> Self {
        LZJDError::Bincode { err }
    }
}

impl From<std::io::Error> for LZJDError {
    fn from(err: std::io::Error) -> Self {
        LZJDError::Io { err }
    }
}

impl<'a> From<&'a str> for LZJDError {
    fn from(msg: &'a str) -> Self {
        LZJDError::Msg {
            msg: msg.to_owned(),
        }
    }
}

pub type Result<T> = std::result::Result<T, LZJDError>;

#[cfg(test)]
mod tests {
    use crate::crc32::CRC32BuildHasher;
    use crate::*;
    use std::f64::EPSILON;

    #[test]
    fn test_optimized_dist() {
        let build_hasher = CRC32BuildHasher;

        let a = b"THIS IS A TEST SEQUENCE";
        let b = b"THIS IS A TEST SEQUENCE";
        let c = b"totally_different";
        let d = b"THIS IS A DIFFERENT TEST SEQUENCE";

        let dict_a = LZDict::from_bytes_stream_lz78(a.iter().cloned(), &build_hasher);
        let dict_b = LZDict::from_bytes_stream_lz78(b.iter().cloned(), &build_hasher);
        let dict_c = LZDict::from_bytes_stream_lz78(c.iter().cloned(), &build_hasher);
        let dict_d = LZDict::from_bytes_stream_lz78(d.iter().cloned(), &build_hasher);

        let dist = dict_a.dist(&dict_b);
        assert!(
            dist.abs() < EPSILON, // dist(a, b) == 0
            "Distance of equal sequences (a and b) should equal 0, was {}",
            dist
        );
        let dist = dict_a.dist(&dict_c);
        assert!(
            (1. - dist).abs() < EPSILON, // dist(a, c) == 1
            "Distance of totally different sequences (a and c) should equal 1, was {}",
            dist
        );
        let dist = dict_a.dist(&dict_d);
        assert!(
            (0.409_090_909_090_909_06 - dist).abs() < EPSILON, // dist(a, d) == 0.409_090_909_090_909_06
            "Distance of a and d should equal 0.40909090909090906, was {}",
            dist
        );
        assert!(
            (dict_a.dist(&dict_d) - dict_d.dist(&dict_a)).abs() < EPSILON, // dist(a,d) == dist(d,a)
            "Distance of a and d should be equal to distance of d and a"
        );
    }
}

//! Defines a wrapper around crc::crc32::Digest, implementing std::hash::Hasher
//! as well as a std::hash::BuildHasher which builds the hasher.
use crc::crc32::{self, Hasher32};

use std::hash::BuildHasher;
use std::hash::Hasher;

/// Wrapper around crc::crc32::Digest which implements std::hash::Hasher
pub struct CRC32Hasher {
    digest: crc::crc32::Digest,
}

impl CRC32Hasher {
    fn new() -> Self {
        Self {
            digest: crc32::Digest::new(crc::crc32::IEEE),
        }
    }
}

impl Hasher for CRC32Hasher {
    fn finish(&self) -> u64 {
        u64::from(self.digest.sum32())
    }

    fn write(&mut self, bytes: &[u8]) {
        Hasher32::write(&mut self.digest, bytes);
    }
}

/// std::hash::BuildHasher that builds CRC32Hashers
#[derive(Clone)]
pub struct CRC32BuildHasher;

impl BuildHasher for CRC32BuildHasher {
    type Hasher = CRC32Hasher;

    fn build_hasher(&self) -> Self::Hasher {
        CRC32Hasher::new()
    }
}

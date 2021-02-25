use fasthash::FastHasher;
use core::hash::BuildHasher;

pub struct Murmur3BuildHasher;

/// std::hash::BuildHasher that builds Murmur3HasherExt hashers
impl BuildHasher for Murmur3BuildHasher {
    type Hasher = fasthash::murmur3::Hasher32;

    fn build_hasher(&self) -> Self::Hasher {
        fasthash::murmur3::Hasher32::new()
    }
}

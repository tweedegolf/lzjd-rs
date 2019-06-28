use fasthash::{murmur3, Murmur3HasherExt};
use std::hash::BuildHasher;

pub struct Murmur3BuildHasher;

/// std::hash::BuildHasher that builds Murmur3HasherExt hashers
impl BuildHasher for Murmur3BuildHasher {
    type Hasher = Murmur3HasherExt;

    fn build_hasher(&self) -> Self::Hasher {
        Murmur3HasherExt::default()
    }
}
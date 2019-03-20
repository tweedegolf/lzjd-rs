use crate::Result;
use core::hash::BuildHasher;
use core::hash::Hasher;
use core::ops::Deref;

/// A sorted list of the k smallest LZSet hashes
#[derive(Debug)]
pub struct LZDict {
    // Once const generics are stablilized, entries can be an array
    // and the crate can become no_std
    entries: Vec<u64>,
}

impl LZDict {
    /// Converts a base64 string into a Vec<u64> and wraps a LZDict around it.
    pub fn from_base64_string(b64: &str) -> Result<Self> {
        let bytes = base64::decode(b64)?;
        let mut entries = vec![];
        for i in 0..bytes.len() / 8 {
            let vec = bytes
                .iter()
                .cloned()
                .skip(i * 8)
                .take(8)
                .fold(vec![], |mut v, b| {
                    v.push(b);
                    v
                });
            entries.push(bincode::deserialize(&vec)?);
        }

        Ok(Self { entries })
    }

    /// Creates a LZ dictionary containing the smallest k hashes
    /// of LZ sequences obtained from seq_iter.
    pub fn from_bytes_stream<I, H>(seq_iter: I, build_hasher: &H, k: usize) -> Self
    where
        I: Iterator<Item = u8>,
        H: BuildHasher,
    {
        let mut entries = Vec::with_capacity(k);
        let mut hasher = build_hasher.build_hasher();

        seq_iter.for_each(|byte| {
            // Update hash
            hasher.write_u8(byte);
            let hash = hasher.finish();

            if let Err(insert_at) = entries.binary_search(&hash) {
                // If entries does not yet contain current hash
                if entries.len() < k {
                    // There's room for another hash without reallocating
                    entries.insert(insert_at, hash); // Insert current hash
                    hasher = build_hasher.build_hasher(); // Reset hasher
                } else if hash < *entries.last().unwrap() {
                    // Current hash is smaller than largest in entries
                    entries.pop(); // Remove greatest hash

                    entries.insert(insert_at, hash); // Insert current hash
                    hasher = build_hasher.build_hasher(); // Reset hasher
                }
            }
            // else it's already in there and we can go on
        });

        LZDict { entries }
    }

    fn intersection_len(&self, other: &Self) -> usize {
        let mut i = 0;
        let mut j = 0;
        let mut len = 0;
        while i < self.len() && j < other.len() {
            let self_entry = self[i];
            let other_entry = other[j];
            if self_entry <= other_entry {
                i += 1;
            }
            if self_entry >= other_entry {
                j += 1;
            }
            if self_entry == other_entry {
                len += 1;
            }
        }
        len
    }

    /// Calculates the jaccard similarity of the entries two dictionaries
    /// which is defined as the length of the intersection over the length of the union.
    pub fn jaccard_similarity(&self, other: &Self) -> f32 {
        let intersection_len = self.intersection_len(other);

        let union_len = self.len() + other.len() - intersection_len;

        intersection_len as f32 / union_len as f32
    }

    /// Encodes the contents of the dictionary to base64 and returns it as a string.
    pub fn to_string(&self) -> String {
        let bytes: Vec<u8> = self
            .iter()
            .map(|hash| bincode::serialize(&hash).unwrap())
            .flatten()
            .collect();
        base64::encode(&bytes)
    }

    /// Calculates the LZ-distance of two LZ Dictionaries
    pub fn dist(&self, other: &LZDict) -> f32 {
        1.0 - self.similarity(other)
    }

    /// Calculates the LZ-similarity of two LZ Dictionaries
    pub fn similarity(&self, other: &LZDict) -> f32 {
        self.jaccard_similarity(other)
    }
}

impl Deref for LZDict {
    type Target = Vec<u64>;

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl From<Vec<u64>> for LZDict {
    fn from(mut entries: Vec<u64>) -> Self {
        entries.sort();
        entries.truncate(1024);
        Self { entries }
    }
}

impl From<LZDict> for Vec<u64> {
    fn from(item: LZDict) -> Self {
        item.entries
    }
}

#[cfg(test)]
mod tests {
    use crate::crc32::CRC32BuildHasher;
    use crate::lz_dict::LZDict;
    use std::f32::EPSILON;
    use std::iter::*;

    fn is_sorted_and_unique<T: PartialOrd>(list: &[T]) -> bool {
        if list.len() <= 1 {
            true
        } else {
            for i in 1..list.len() {
                if list[i - 1] >= list[i] {
                    return false;
                }
            }

            true
        }
    }

    #[test]
    fn test_from_bytes_iter() {
        let sequence = b"TESTSEQUENCETESTTESTTTTTEESSTT".to_vec();
        let k = 10;
        let build_hasher = CRC32BuildHasher;
        let lz_dict = LZDict::from_bytes_stream(sequence.iter().cloned(), &build_hasher, k);

        assert!(
            is_sorted_and_unique(&lz_dict),
            "Entries of dictionary are either not sorted or not unique"
        );

        assert!(lz_dict.len() <= k);
    }

    #[test]
    fn test_jaccard_similarity() {
        const A_ENTRIES: [u64; 4] = [0, 1, 2, 3];
        const B_ENTRIES: [u64; 3] = [0, 1, 2];
        const C_ENTRIES: [u64; 4] = [1, 2, 3, 4];
        const D_ENTRIES: [u64; 0] = [];
        const E_ENTRIES: [u64; 4] = [4, 5, 6, 7];
        const F_ENTRIES: [u64; 5] = [0, 1, 2, 3, 5];

        const UNION_A_A_LEN: usize = 4;
        const UNION_A_B_LEN: usize = 4;
        const UNION_A_C_LEN: usize = 5;
        const UNION_A_D_LEN: usize = 4;
        const UNION_A_E_LEN: usize = 8;
        const UNION_A_F_LEN: usize = 5;

        const INTERSECTION_A_A_LEN: usize = 4;
        const INTERSECTION_A_B_LEN: usize = 3;
        const INTERSECTION_A_C_LEN: usize = 3;
        const INTERSECTION_A_D_LEN: usize = 0;
        const INTERSECTION_A_E_LEN: usize = 0;
        const INTERSECTION_A_F_LEN: usize = 4;

        let a = LZDict {
            entries: A_ENTRIES.to_vec(),
        };
        let b = LZDict {
            entries: B_ENTRIES.to_vec(),
        };
        let c = LZDict {
            entries: C_ENTRIES.to_vec(),
        };
        let d = LZDict {
            entries: D_ENTRIES.to_vec(),
        };
        let e = LZDict {
            entries: E_ENTRIES.to_vec(),
        };
        let f = LZDict {
            entries: F_ENTRIES.to_vec(),
        };

        assert!(
            (a.jaccard_similarity(&a) - INTERSECTION_A_A_LEN as f32 / UNION_A_A_LEN as f32).abs()
                < EPSILON
        );
        assert!(
            (a.jaccard_similarity(&b) - INTERSECTION_A_B_LEN as f32 / UNION_A_B_LEN as f32).abs()
                < EPSILON
        );
        assert!(
            (a.jaccard_similarity(&c) - INTERSECTION_A_C_LEN as f32 / UNION_A_C_LEN as f32).abs()
                < EPSILON
        );
        assert!(
            (a.jaccard_similarity(&d) - INTERSECTION_A_D_LEN as f32 / UNION_A_D_LEN as f32).abs()
                < EPSILON
        );
        assert!(
            (a.jaccard_similarity(&e) - INTERSECTION_A_E_LEN as f32 / UNION_A_E_LEN as f32).abs()
                < EPSILON
        );
        assert!(
            (a.jaccard_similarity(&f) - INTERSECTION_A_F_LEN as f32 / UNION_A_F_LEN as f32).abs()
                < EPSILON
        );
    }
}

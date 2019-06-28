use crate::Result;
use core::hash::BuildHasher;
use core::hash::Hasher;
use core::ops::Deref;
use std::fmt::Debug;
use std::collections::HashSet;

/// A sorted list of the k smallest LZSet hashes
#[derive(Debug)]
pub struct LZDict {
    // Once const generics are stablilized, entries can be an array
    // and the crate can become no_std
    entries: Vec<i64>,
}

impl LZDict {
    /// Converts a base64 string into a Vec<i64> and wraps a LZDict around it.
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
    /// Based on LZ78 as described in https://en.wikipedia.org/wiki/LZ77_and_LZ78#LZ78
    pub fn from_bytes_stream_lz78<I, H>(seq_iter: I, build_hasher: &H) -> Self
        where
            I: Iterator<Item=u8>,
            H: BuildHasher,
    {
        let mut dict: Vec<(usize, u8)> = Vec::new();
        let mut last_matching_index: usize = 0;
        dict.push((0, 0));

        for item in seq_iter {
            if let Some(index) = dict.iter().position(
                |(lmi, i)| lmi == &last_matching_index && i == &item
            ) {
                last_matching_index = index;
            } else {
                dict.push((last_matching_index, item));
                last_matching_index = 0;
            }
        }

        let mut hashes: Vec<i64> = Vec::new();
        let mut hasher = build_hasher.build_hasher();

        for i in 1..dict.len() {
            Self::hash_entry(i, &dict, &mut hasher);
            let hash = hasher.finish();
            let serializedHash: &[u8]  = &bincode::serialize(&hash).unwrap();
            let hash_i64: i64 = bincode::deserialize(serializedHash).unwrap();
            hasher = build_hasher.build_hasher();

            if let Err(insert_at) = hashes.binary_search(&hash_i64) {
                if hashes.len() < 1024 {
                    hashes.insert(insert_at, hash_i64); // Insert current hash
                } else if hash_i64 < *hashes.last().unwrap() {
                    hashes.pop(); // Remove greatest hash

                    hashes.insert(insert_at, hash_i64); // Insert current hash
                }
            }
        }

        LZDict { entries: hashes }
    }

    fn hash_entry<H: Hasher>(index: usize, dict: &Vec<(usize, u8)>, hasher: &mut H) {
        if index == 0 {
            return;
        }
        let entry = dict[index];
        Self::hash_entry(entry.0, dict, hasher);
        hasher.write_u8(entry.1);
    }

    pub fn from_bytes_stream<I, H>(seq_iter: I, build_hasher: &H) -> Self
        where
            I: Iterator<Item=u8>,
            H: BuildHasher,
    {
        let mut dict = HashSet::new();
        let mut hasher = build_hasher.build_hasher();

        for byte in seq_iter {
            hasher.write_u8(byte);
            let hash = hasher.finish();
            let serializedHash: &[u8]  = &bincode::serialize(&hash).unwrap();
            let hash_i64: i64 = bincode::deserialize(serializedHash).unwrap();
            if dict.insert(hash_i64) {
                hasher = build_hasher.build_hasher();
            }
        }

        let mut dict: Vec<_> = dict.iter().cloned().collect();
        dict.sort();

        LZDict { entries: dict.iter().cloned().take(1000).collect() }
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
    pub fn jaccard_similarity(&self, other: &Self) -> f64 {
        let intersection_len = self.intersection_len(other);

        let union_len = self.len() + other.len() - intersection_len;

        intersection_len as f64 / union_len as f64
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
    pub fn dist(&self, other: &LZDict) -> f64 {
        1.0 - self.similarity(other)
    }

    /// Calculates the LZ-similarity of two LZ Dictionaries
    pub fn similarity(&self, other: &LZDict) -> f64 {
        self.jaccard_similarity(other)
    }
}

impl Deref for LZDict {
    type Target = Vec<i64>;

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl From<Vec<i64>> for LZDict {
    fn from(mut entries: Vec<i64>) -> Self {
        entries.sort();
        entries.truncate(1024);
        Self { entries }
    }
}

impl From<LZDict> for Vec<i64> {
    fn from(item: LZDict) -> Self {
        item.entries
    }
}

#[cfg(test)]
mod tests {
    use crate::crc32::CRC32BuildHasher;
    use crate::lz_dict::LZDict;
    use std::f64::EPSILON;
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
        let sequence = b"TESTSEQUENCETESTTESTTTTTEESSTT";
        let k = 10;
        let build_hasher = CRC32BuildHasher;
        let lz_dict = LZDict::from_bytes_stream(sequence.iter().cloned(), &build_hasher);

        assert!(
            is_sorted_and_unique(&lz_dict),
            "Entries of dictionary are either not sorted or not unique"
        );

        assert!(lz_dict.len() <= k);
    }

    #[test]
    fn test_jaccard_similarity() {
        const A_ENTRIES: [i64; 4] = [0, 1, 2, 3];
        const B_ENTRIES: [i64; 3] = [0, 1, 2];
        const C_ENTRIES: [i64; 4] = [1, 2, 3, 4];
        const D_ENTRIES: [i64; 0] = [];
        const E_ENTRIES: [i64; 4] = [4, 5, 6, 7];
        const F_ENTRIES: [i64; 5] = [0, 1, 2, 3, 5];

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
            (a.jaccard_similarity(&a) - INTERSECTION_A_A_LEN as f64 / UNION_A_A_LEN as f64).abs()
                < EPSILON
        );
        assert!(
            (a.jaccard_similarity(&b) - INTERSECTION_A_B_LEN as f64 / UNION_A_B_LEN as f64).abs()
                < EPSILON
        );
        assert!(
            (a.jaccard_similarity(&c) - INTERSECTION_A_C_LEN as f64 / UNION_A_C_LEN as f64).abs()
                < EPSILON
        );
        assert!(
            (a.jaccard_similarity(&d) - INTERSECTION_A_D_LEN as f64 / UNION_A_D_LEN as f64).abs()
                < EPSILON
        );
        assert!(
            (a.jaccard_similarity(&e) - INTERSECTION_A_E_LEN as f64 / UNION_A_E_LEN as f64).abs()
                < EPSILON
        );
        assert!(
            (a.jaccard_similarity(&f) - INTERSECTION_A_F_LEN as f64 / UNION_A_F_LEN as f64).abs()
                < EPSILON
        );
    }
}

#![feature(test)]

extern crate test;

#[cfg(test)]
mod benches {
    use ::lzjd::LZDict;
    use ::lzjd::crc32::CRC32BuildHasher;
    use rand::prelude::*;
    use test::Bencher;

    fn generate_byte_sequence() -> Vec<u8> {
        let parts: Vec<[u8; 32]> = (0..10000)
            .map(|_| {
                let mut part = [0u8; 32];
                rand::thread_rng().fill(&mut part);
                part
            })
            .collect();
        parts.iter().flatten().cloned().collect()
    }

    #[bench]
    fn bench_dist(b: &mut Bencher) {
        let build_hasher = CRC32BuildHasher;

        let seq_a = generate_byte_sequence();
        let seq_b = generate_byte_sequence();

        b.iter(move || {
            let dict_a = LZDict::from_bytes_stream(seq_a.iter().cloned(), &build_hasher, 1024);
            let dict_b = LZDict::from_bytes_stream(seq_b.iter().cloned(), &build_hasher, 1024);

            dict_a.dist(&dict_b);
        });
    }
}

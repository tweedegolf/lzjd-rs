#[macro_use]
extern crate criterion;

use criterion::Criterion;
use ::lzjd::LZDict;
use ::lzjd::crc32::CRC32BuildHasher;
use rand::prelude::*;

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

fn bench_dist(c: &mut Criterion) {
   
    c.bench_function("LZDict::from_bytes_stream", |b| {
        let build_hasher = CRC32BuildHasher;

        let seq_a = generate_byte_sequence();
        let seq_b = generate_byte_sequence();
        b.iter(move || {
            let dict_a = LZDict::from_bytes_stream(seq_a.iter().cloned(), &build_hasher);
            let dict_b = LZDict::from_bytes_stream(seq_b.iter().cloned(), &build_hasher);

            dict_a.dist(&dict_b);
        })
    });
}

criterion_group!(benches, bench_dist);
criterion_main!(benches);

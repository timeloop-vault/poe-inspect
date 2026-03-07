
/// MurmurHash64A — used by PoE since patch 3.21.2 for bundle file path hashing.
/// Seed is always 0x1337b33f for file path lookups.
pub fn murmurhash64a(data: &[u8], seed: u64) -> u64 {
    const M: u64 = 0xc6a4a7935bd1e995;
    const R: u32 = 47;

    let len = data.len();
    let mut h: u64 = seed ^ (len as u64).wrapping_mul(M);

    let nblocks = len / 8;
    for i in 0..nblocks {
        let mut k = u64::from_le_bytes(data[i * 8..(i + 1) * 8].try_into().unwrap());
        k = k.wrapping_mul(M);
        k ^= k >> R;
        k = k.wrapping_mul(M);

        h ^= k;
        h = h.wrapping_mul(M);
    }

    let tail = &data[nblocks * 8..];
    let remaining = len & 7;

    if remaining >= 7 { h ^= (tail[6] as u64) << 48; }
    if remaining >= 6 { h ^= (tail[5] as u64) << 40; }
    if remaining >= 5 { h ^= (tail[4] as u64) << 32; }
    if remaining >= 4 { h ^= (tail[3] as u64) << 24; }
    if remaining >= 3 { h ^= (tail[2] as u64) << 16; }
    if remaining >= 2 { h ^= (tail[1] as u64) << 8; }
    if remaining >= 1 {
        h ^= tail[0] as u64;
        h = h.wrapping_mul(M);
    }

    h ^= h >> R;
    h = h.wrapping_mul(M);
    h ^= h >> R;

    h
}

/// Hash a file path for bundle index lookup (3.21.2+ format).
/// Lowercases the path, no suffix, uses MurmurHash64A with seed 0x1337b33f.
pub fn filepath_hash(data: String) -> u64 {
    let lowercase = data.to_lowercase();
    murmurhash64a(lowercase.as_bytes(), 0x1337b33f)
}

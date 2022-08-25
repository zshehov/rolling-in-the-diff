// TODO: implement marshaling to binary
// probably use https://github.com/bincode-org/bincode

use std::collections::HashMap;
use std::hash::Hash;

use rayon::iter::{IndexedParallelIterator, ParallelIterator};
use rayon::slice::ParallelSlice;

use crate::{ChunkNumber, Signature};
use crate::rolling_checksum::RollingChecksum;
use crate::strong_hash::StrongHash;

pub fn generate_signature<R, S>(content: &[u8]) -> Signature<R::ChecksumType, S::HashType> where
    R: RollingChecksum,
    <R as RollingChecksum>::ChecksumType: Eq + Hash,
    S: StrongHash,
    <R as RollingChecksum>::ChecksumType: Send + Copy,
    <S as StrongHash>::HashType: Send,
{
    // todo!("estimate chunk size based on content length");
    let chunk_size = 4096;

    // calculate checksum + hash for each chunk in parallel
    let checksum_hash_tuples: Vec<(usize, R::ChecksumType, S::HashType)> = content.par_chunks(chunk_size).enumerate().map(|(chunk_number, chunk)| {
        let checksum = R::new(chunk).checksum();
        let hash = S::hash(chunk);
        return (chunk_number, checksum, hash);
    }).collect();

    let mut signature_map: HashMap<R::ChecksumType, Vec<(S::HashType, ChunkNumber)>> =
        HashMap::with_capacity(checksum_hash_tuples.len());

    let chunk_count = checksum_hash_tuples.len();
    // go through all chunks sequentially - if this is too slow,
    // concurrent hash maps are an option that might speed things up
    for (chunk_number, checksum, hash) in checksum_hash_tuples {
        if !signature_map.contains_key(&checksum) {
            signature_map.insert(checksum, Vec::with_capacity(1));
        }
        signature_map.get_mut(&checksum).unwrap().push((hash, chunk_number as ChunkNumber));
    }

    return Signature {
        checksum_to_hashes: signature_map,
        chunk_size,
        chunk_count,
    };
}

#[cfg(test)]
mod test {
    use adler32::RollingAdler32 as actual_adler32;

    use crate::rolling_checksum::rolling_adler32::RollingAdler32;
    use crate::strong_hash::md5::Md5Sum;
    use crate::strong_hash::StrongHash;

    use super::*;

    #[test]
    fn test_generate_signature() {
        let content = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let signature = generate_signature::<RollingAdler32, Md5Sum>(&content);

        for (chunk_number, chunk) in content.chunks(signature.chunk_size).enumerate() {
            let checksum = actual_adler32::from_buffer(chunk).hash();
            assert!(signature.checksum_to_hashes.contains_key(&checksum));
            let chunks = signature.checksum_to_hashes.get(&checksum).unwrap();

            assert!(chunks.contains(&(Md5Sum::hash(chunk), chunk_number as ChunkNumber)))
        }
    }
}

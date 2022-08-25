// TODO: implement marshaling to binary
// probably use https://github.com/bincode-org/bincode

use std::collections::HashMap;
use std::hash::Hash;

use rayon::iter::{IndexedParallelIterator, ParallelIterator};
use rayon::slice::ParallelSlice;

use crate::{ChunkNumber, Signature};
use crate::delta_generation::Error;

pub fn generate_signature<R, S>(content: &[u8]) -> Result<Signature<R::ChecksumType, S::HashType>, Error> where
    R: super::RollingChecksum,
    <R as super::RollingChecksum>::ChecksumType: Eq + Hash,
    S: super::StrongHash,
    <R as super::RollingChecksum>::ChecksumType: Send + Copy,
    <S as super::StrongHash>::HashType: Send,
{
    // todo!("estimate chunk size based on content length");
    let chunk_size = 3;

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

    return Ok(Signature {
        checksum_to_hashes: signature_map,
        chunk_size,
        chunk_count,
    });
}

#[cfg(test)]
mod test {
    use adler32::RollingAdler32 as actual_adler32;

    use crate::StrongHash;

    use super::*;

    struct TestHash {}

    impl StrongHash for TestHash {
        type HashType = u32;

        // hash is just the sum of the bytes
        fn hash(data: &[u8]) -> Self::HashType {
            // sum can only sum to the same type, which makes it unusable for u8 - go for a raw fold here
            return data.iter().fold(0, |acc, &next| acc + next as u32);
        }
    }

    #[test]
    fn test_generate_signature() {
        let content = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let signature = generate_signature::<crate::delta_generation::rolling_adler32::RollingAdler32, TestHash>(&content).unwrap();

        for (chunk_number, chunk) in content.chunks(signature.chunk_size).enumerate() {
            let checksum = actual_adler32::from_buffer(chunk).hash();
            assert!(signature.checksum_to_hashes.contains_key(&checksum));
            let chunks = signature.checksum_to_hashes.get(&checksum).unwrap();

            assert!(chunks.contains(&(TestHash::hash(chunk), chunk_number as ChunkNumber)))
        }
    }
}

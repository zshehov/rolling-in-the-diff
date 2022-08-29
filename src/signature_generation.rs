use std::collections::HashMap;
use std::hash::Hash;

use log::info;
use rayon::iter::{IndexedParallelIterator, ParallelIterator};
use rayon::slice::ParallelSlice;

use crate::rolling_checksum::RollingChecksum;
use crate::strong_hash::StrongHash;
use crate::{ChunkNumber, Signature};

pub fn generate_signature<R, S>(content: &[u8]) -> Signature<R::ChecksumType, S::HashType>
where
    R: RollingChecksum,
    <R as RollingChecksum>::ChecksumType: Eq + Hash,
    S: StrongHash,
    <R as RollingChecksum>::ChecksumType: Send + Copy,
    <S as StrongHash>::HashType: Send,
{
    let version = crate::VERSION.unwrap_or(crate::DEFAULT_VERSION).to_string();
    if content.is_empty() {
        return Signature {
            checksum_to_hashes: HashMap::<R::ChecksumType, Vec<(S::HashType, ChunkNumber)>>::new(),
            chunk_size: 0,
            chunk_count: 0,
            version,
        };
    }
    let chunk_size = determine_chunk_size::<R::ChecksumType, S::HashType>(content.len());
    info!(
        "content len: {} chunk count: {}; chunk size: {}",
        content.len(),
        (content.len() as f64 / (chunk_size as f64)).ceil(),
        chunk_size
    );

    // calculate checksum + hash for each chunk in parallel
    let checksum_hash_tuples: Vec<(usize, R::ChecksumType, S::HashType)> = content
        .par_chunks(chunk_size)
        .enumerate()
        .map(|(chunk_number, chunk)| {
            let checksum = R::new(chunk).checksum();
            let hash = S::hash(chunk);
            (chunk_number, checksum, hash)
        })
        .collect();

    let mut signature_map: HashMap<R::ChecksumType, Vec<(S::HashType, ChunkNumber)>> =
        HashMap::with_capacity(checksum_hash_tuples.len());

    let chunk_count = checksum_hash_tuples.len();
    // go through all chunks sequentially - if this is too slow,
    // concurrent hash maps are an option that might speed things up
    for (chunk_number, checksum, hash) in checksum_hash_tuples {
        signature_map
            .entry(checksum)
            .or_insert_with(|| Vec::with_capacity(1))
            .push((hash, chunk_number as ChunkNumber));
    }

    Signature {
        checksum_to_hashes: signature_map,
        chunk_size,
        chunk_count,
        version,
    }
}

const MAGIC_CHUNK_COUNT: usize = (1 << 10) << 2;

///
/// Determines a "good" chunk size based on the content length
///
/// ```
/// use rolling_in_the_diff::signature_generation::determine_chunk_size;
/// use rolling_in_the_diff::ChunkNumber;
///
///
/// // when the overhead of hashes is bigger than the content
/// assert_eq!(determine_chunk_size::<u64, u64>(6), 6);
///
/// let overhead = 1 + 1 + std::mem::size_of::<ChunkNumber>();
/// let content_len = 20 * overhead;
/// // can fit 20 overheads -> the next smaller power of 2 == 16
/// assert_eq!(determine_chunk_size::<u8, u8>(content_len), content_len / 16);
/// ```
pub fn determine_chunk_size<R, S>(content_len: usize) -> usize {
    let overhead_per_chunk =
        std::mem::size_of::<R>() + std::mem::size_of::<S>() + std::mem::size_of::<ChunkNumber>();

    let mut chunk_count = MAGIC_CHUNK_COUNT;
    while chunk_count > 0 {
        let overhead = chunk_count * overhead_per_chunk;
        if overhead >= content_len {
            chunk_count >>= 1;
        } else {
            break;
        }
    }
    if chunk_count == 0 {
        // the overhead of having signatures will defeat the purpose of having multiple chunks
        // just having 1 chunk will be better
        return content_len;
    }

    // TODO: maybe have some chunk_size cap here? For 10GB and MAGIC_CHUNK_COUNT=4k -> 2.5MB seems reasonable for now
    content_len / chunk_count
}

#[cfg(test)]
mod test {
    use adler32::RollingAdler32 as actual_adler32;
    use test_case::test_case;

    use crate::rolling_checksum::rolling_adler32::RollingAdler32;
    use crate::strong_hash::md5::Md5Sum;
    use crate::strong_hash::StrongHash;

    use super::*;

    #[test_case(10, 1; "when content is small and even")]
    #[test_case(9, 1; "when content is small and odd")]
    #[test_case(1 << 10, 2; "when content is big enough for multiple chunks signature")]
    fn test_generate_signature(content_len: usize, at_least_chunk_count: usize) {
        let content: Vec<u8> = (0..content_len).map(|x| x as u8).collect();
        let signature = generate_signature::<RollingAdler32, Md5Sum>(&content);

        assert!(
            signature.chunk_count >= at_least_chunk_count,
            "{} < {}",
            signature.chunk_count,
            at_least_chunk_count
        );
        for (chunk_number, chunk) in content.chunks(signature.chunk_size).enumerate() {
            let checksum = actual_adler32::from_buffer(chunk).hash();
            assert!(signature.checksum_to_hashes.contains_key(&checksum));
            let chunks = signature.checksum_to_hashes.get(&checksum).unwrap();

            assert!(chunks.contains(&(Md5Sum::hash(chunk), chunk_number as ChunkNumber)))
        }
    }

    #[test]
    fn test_generate_signature_of_empty_content() {
        let content: &[u8] = &[];
        let signature = generate_signature::<RollingAdler32, Md5Sum>(content);
        assert_eq!(signature.chunk_count, 0);
        assert_eq!(signature.chunk_size, 0);
        assert!(signature.checksum_to_hashes.is_empty());
    }

    struct DummyRolling {}

    const DUMMY_CHECKSUM: u8 = 69;

    impl RollingChecksum for DummyRolling {
        type ChecksumType = u8;
        fn new(_: &[u8]) -> Self {
            Self {}
        }
        fn checksum(&self) -> Self::ChecksumType {
            DUMMY_CHECKSUM
        }
        fn push_byte(&mut self, _: u8) {}
        fn pop_byte(&mut self, _: u8, _: usize) {}
    }

    struct DummyHash {}

    impl StrongHash for DummyHash {
        type HashType = u32;
        fn hash(_: &[u8]) -> Self::HashType {
            420
        }
    }

    #[test]
    fn test_generate_signature_with_repeating_checksums() {
        let content = [0; 420];

        let signature = generate_signature::<DummyRolling, DummyHash>(&content);

        assert!(signature.chunk_count > 1);
        assert_eq!(signature.checksum_to_hashes.keys().len(), 1);
        assert_eq!(
            signature
                .checksum_to_hashes
                .get(&DUMMY_CHECKSUM)
                .unwrap()
                .len(),
            signature.chunk_count
        );
    }
}

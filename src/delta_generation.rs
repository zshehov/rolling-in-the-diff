use std::cmp::min;
use std::fmt::Debug;
use std::hash::Hash;

use bitvec::bitvec;
use indicatif::{ProgressBar, ProgressStyle};
use log::{error, info};
use serde::{Deserialize, Serialize};

use crate::ChunkNumber;
use crate::delta_generation::DeltaToken::{Added, Removed, Reused};
use crate::rolling_checksum::RollingChecksum;
use crate::strong_hash::StrongHash;

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum DeltaToken<'a, S> where
    S: PartialEq + Debug,
{
    Reused(
        ChunkNumber /* chunk number in old file */,
        S /* strong hash over the content for the patch operation to use*/,
    ),
    Added(&'a [u8] /* new data */),
    Removed(
        ChunkNumber,
    ),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Delta<'a, S> where
    S: Eq + PartialEq + Debug,
{
    #[serde(borrow)]
    pub tokens: Vec<DeltaToken<'a, S>>,
    pub chunk_size: u64,
    pub base_content_version: u64,
}

pub fn generate_delta<'a, R, S>(
    old_signature: &crate::Signature<R::ChecksumType, S::HashType>,
    new_content: &'a [u8],
) -> Delta<'a, S::HashType> where
    R: RollingChecksum,
    <R as RollingChecksum>::ChecksumType: Eq + Hash,
    S: StrongHash,
    <S as StrongHash>::HashType: Eq,
{
    let mut reused_chunks = bitvec![0; old_signature.chunk_count];

    let mut delta = Delta {
        tokens: Vec::with_capacity(old_signature.chunk_count),
        chunk_size: old_signature.chunk_size as u64,
        base_content_version: 2,
    };
    let mut left = 0;

    let progress = ProgressBar::new(new_content.len() as u64);
    progress.set_style(ProgressStyle::default_bar().template("{msg} {bar} {bytes}/{total_bytes}").unwrap());
    progress.set_message("Going through new content:");
    let mut reused_count = 0;
    loop {
        match find_reused_chunk::<R, S>(old_signature, &new_content[left..]) {
            Some(reused_chunk) => {
                if reused_chunk.bytes_until_reused > 0 {
                    delta.tokens.push(Added(&new_content[left..left + reused_chunk.bytes_until_reused]));
                    left += reused_chunk.bytes_until_reused;
                }
                delta.tokens.push(Reused(reused_chunk.chunk_number, reused_chunk.chunk_strong_hash));
                left += reused_chunk.reused_chunk_size;
                if reused_chunk.chunk_number >= old_signature.chunk_count as ChunkNumber {
                    // that might very well mean an invalid signature file
                    error!("signature contains a chunk number out of the chunk count: {} >= {}",
                        reused_chunk.chunk_number,
                        old_signature.chunk_count);
                    continue;
                }
                reused_chunks.set(reused_chunk.chunk_number as usize, true);
                reused_count += 1;
                progress.set_position(left as u64);
            }
            None => {
                // couldn't find a single match until the end of the new content - finish up the delta
                // note: empty new_content with [0..] is a valid usage
                if !new_content[left..].is_empty() {
                    delta.tokens.push(Added(&new_content[left..]));
                }
                // fill up all the removed chunks at the end
                for i in 0..old_signature.chunk_count {
                    if let Some(&true) = reused_chunks.get(i).as_deref() {
                        continue;
                    }
                    delta.tokens.push(Removed(i as ChunkNumber));
                }
                progress.finish();
                info!("reused chunks: {}", reused_count);
                return delta;
            }
        }
    }
}

struct ReusedChunkDescriptor<T> {
    bytes_until_reused: usize,
    reused_chunk_size: usize,
    chunk_number: ChunkNumber,
    chunk_strong_hash: T,
}

fn find_reused_chunk<R, S>(
    old_signature: &crate::Signature<R::ChecksumType, S::HashType>,
    new_content: &[u8],
) -> Option<ReusedChunkDescriptor<S::HashType>> where
    R: RollingChecksum,
    <R as RollingChecksum>::ChecksumType: Eq + Hash,
    S: StrongHash,
{
    // it's possible that the original file includes a non-chunk-aligned chunk at the end
    // and it can be matched by a less-than-old_signature.chunk_size from the new content
    let mut chunk_after_end = min(old_signature.chunk_size, new_content.len());

    let mut rolling_checksum = R::new(&new_content[..chunk_after_end]);
    let mut chunk_start = 0;

    loop {
        if chunk_after_end - chunk_start == 0 {
            return None;
        }
        let checksum = rolling_checksum.checksum();

        if let Some(strong_hashes) = old_signature.quick_query(&checksum) {
            let hash = S::hash(&new_content[chunk_start..chunk_after_end]);

            for (signature_hash, chunk_number) in strong_hashes {
                let signature_hash = *signature_hash;
                if signature_hash == hash {
                    return Some(ReusedChunkDescriptor {
                        bytes_until_reused: chunk_start,
                        reused_chunk_size: chunk_after_end - chunk_start,
                        chunk_number: *chunk_number,
                        chunk_strong_hash: signature_hash,
                    });
                }
            }
        }
        rolling_checksum.pop_byte(new_content[chunk_start], chunk_after_end - chunk_start);
        if chunk_after_end < new_content.len() {
            rolling_checksum.push_byte(new_content[chunk_after_end]);
            chunk_after_end += 1;
        }
        chunk_start += 1;
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::iter::zip;

    use adler32::RollingAdler32 as actual_adler32;
    use test_case::test_case;

    use crate::rolling_checksum::rolling_adler32::RollingAdler32;
    use crate::Signature;
    use crate::strong_hash::md5::Md5Sum;

    use super::*;

    #[test_case(
    3, & [1, 2, 3, 4, 5, 6], & [0, 1, 2, 4, 5, 6] =>
    vec ! [
    Added(& [0, 1, 2]),
    Reused(1, Md5Sum::hash(& [4, 5, 6])),
    Removed(0),
    ]; "chunks are perfectly aligned")]
    #[test_case(
    3, & [1, 2, 3, 4, 5], & [0, 1, 2, 4, 5] =>
    vec ! [
    Added(& [0, 1, 2]),
    Reused(1, Md5Sum::hash(& [4, 5])),
    Removed(0),
    ];
    "last chunk is not full")]
    #[test_case(
    3, & [1, 2, 3, 4, 5, 6], & [4, 5, 6, 1, 2, 3] =>
    vec ! [
    Reused(1, Md5Sum::hash(& [4, 5, 6])),
    Reused(0, Md5Sum::hash(& [1, 2, 3])),
    ];
    "full chunks are swapped in the new version")]
    #[test_case(
    3, & [1, 2, 3, 4, 5], & [4, 5, 1, 2, 3] =>
    vec ! [
    Added(& [4, 5]),
    Reused(0, Md5Sum::hash(& [1, 2, 3])),
    Removed(1),
    ];
    "chunks are swapped in the new version with an non-full chunk")]
    #[test_case(
    3, & [1, 2, 3], & [] =>
    vec ! [
    Removed(0),
    ];
    "new content is empty")]
    fn test_generate_delta(chunk_size: usize, old_content: &'static [u8], new_content: &'static [u8]) -> Vec<DeltaToken<'static, <Md5Sum as StrongHash>::HashType>> {
        let mut signature_map: HashMap<u32, Vec<(<Md5Sum as StrongHash>::HashType, ChunkNumber)>> = HashMap::new();

        for (chunk_num, chunk) in old_content.chunks(chunk_size).enumerate() {
            assert_eq!(signature_map.insert(
                actual_adler32::from_buffer(chunk).hash(),
                vec![(Md5Sum::hash(chunk), chunk_num as ChunkNumber)],
            ), None);
        }

        let signature = Signature {
            checksum_to_hashes: signature_map,
            chunk_count: old_content.chunks(chunk_size).len(),
            chunk_size,
        };
        return generate_delta::<RollingAdler32, Md5Sum>(&signature, new_content).tokens;
    }

    #[test]
    fn test_generate_delta_with_empty_old_signature() {
        let signature = Signature {
            checksum_to_hashes: HashMap::<u32, Vec<(<Md5Sum as StrongHash>::HashType, ChunkNumber)>>::new(),
            chunk_count: 0,
            chunk_size: 0,
        };

        let new_content = [
            1, 2, 3,
        ];

        let delta = generate_delta::<RollingAdler32, Md5Sum>(&signature, &new_content);

        let expected_tokens = vec![
            Added(&[1, 2, 3]),
        ];

        assert_eq!(delta.tokens.len(), expected_tokens.len());
        zip(delta.tokens.iter(), expected_tokens.iter()).
            for_each(
                |(actual, expected)| {
                    assert_eq!(actual, expected)
                }
            );
    }
}


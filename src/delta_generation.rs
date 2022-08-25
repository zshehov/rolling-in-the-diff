use std::cmp::min;
use std::fmt::Debug;
use std::hash::Hash;

use bitvec::bitvec;

use crate::ChunkNumber;
use crate::delta_generation::DeltaToken::{Added, Removed, Reused};
use crate::rolling_checksum::RollingChecksum;
use crate::strong_hash::StrongHash;

#[derive(Debug)]
pub enum Error {}

#[derive(PartialEq, Eq, Debug)]
enum DeltaToken<'a, S> where
    S: /*Decode + Encode + */ PartialEq + Debug,
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

pub struct Delta<'a, S> where
    S: /*Decode + Encode + */ PartialEq + Debug,
{
    tokens: Vec<DeltaToken<'a, S>>,
}

pub fn generate_delta<R, S>(
    old_signature: crate::Signature<R::ChecksumType, S::HashType>,
    new_content: &[u8],
) -> Result<Delta<S::HashType>, Error> where
    R: RollingChecksum,
    <R as RollingChecksum>::ChecksumType: Eq + Hash,
    S: StrongHash,
{
    let mut reused_chunks = bitvec![0; old_signature.chunk_count];

    let mut delta = Delta { tokens: Vec::with_capacity(old_signature.chunk_count) };
    let mut left = 0;

    loop {
        match find_reused_chunk::<R, S>(&old_signature, &new_content[left..]) {
            Some(reused_chunk) => {
                if reused_chunk.bytes_until_reused > 0 {
                    delta.tokens.push(Added(&new_content[left..left + reused_chunk.bytes_until_reused]));
                    left += reused_chunk.bytes_until_reused;
                }
                delta.tokens.push(Reused(reused_chunk.chunk_number, reused_chunk.chunk_strong_hash));
                left += reused_chunk.reused_chunk_size;
                if reused_chunk.chunk_number >= old_signature.chunk_count as ChunkNumber {
                    // that might very well mean an invalid signature file
                    // TODO: decide on how should this be handled - for now just hide this error
                    continue;
                }
                reused_chunks.set(reused_chunk.chunk_number as usize, true);
            }
            None => {
                // couldn't find a single match until the end of the new content - finish up the delta
                if left < new_content.len() - 1 {
                    delta.tokens.push(Added(&new_content[left..]));
                }
                // fill up all the removed chunks at the end
                for i in 0..old_signature.chunk_count {
                    if let Some(&true) = reused_chunks.get(i).as_deref() {
                        continue;
                    }
                    delta.tokens.push(Removed(i as ChunkNumber));
                }
                return Ok(delta);
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
        if chunk_after_end - chunk_start <= 0 {
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

    use crate::rolling_checksum::rolling_adler32::RollingAdler32;
    use crate::Signature;
    use crate::strong_hash::md5::Md5Sum;

    use super::*;

    #[test]
    fn test_generate_delta() {
        let old_content = [
            1, 2, 3   /* <- chunk 0 */,
            4, 5, 6   /* <- chunk 1 */,
            7, 8, 9   /* <- chunk 2 */,
            10, 11, 12/* <- chunk 3 */,
            13        /* <- chunk 4 */];

        let chunk_size = 3;

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

        let new_content = [
            21, 22, 23 /* <- totally new */,
            1, 2 /*, 3,   <- modified with deletion */,
            4, 5, 6    /* <- reused */,
            /*7, 8, 9     <- removed */
            10, 11, 200/* <- modified */,
            13         /* <- the forsaken chunk */];

        let delta = generate_delta::<RollingAdler32, Md5Sum>(signature, &new_content).unwrap();

        let expected_tokens = vec![
            Added(&[21, 22, 23, 1, 2]),
            Reused(1, Md5Sum::hash(&[4, 5, 6])),
            Added(&[10, 11, 200]),
            Reused(4, Md5Sum::hash(&[13])),
            Removed(0),
            Removed(2),
            Removed(3),
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



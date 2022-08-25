use std::fmt::Debug;
use std::hash::Hash;

use crate::chunk_number;
use crate::delta_generator::DeltaToken::{Added, Reused};

mod adler32;

#[derive(Debug)]
pub enum Error {
    NoInput,
    FailedRead(std::io::Error),
}

pub trait RollingChecksum {
    type ChecksumType;

    fn new(initial_data: &[u8]) -> Self;
    fn checksum(&self) -> Self::ChecksumType;

    fn roll_window(&mut self, old_byte: u8, new_byte: u8);
}

pub trait StrongHash {
    type HashType: PartialEq + Debug + Copy;

    fn hash(data: &[u8]) -> Self::HashType;
}

enum DeltaToken<'a, S> where
    S: /*Decode + Encode + */ PartialEq + Debug,
{
    Reused(
        chunk_number /* chunk number in old file */,
        S /* strong hash over the content for the patch operation to use*/,
    ),
    Added(&'a [u8] /* new data */),
    Removed(
        chunk_number,
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
    let mut delta = Delta { tokens: Vec::with_capacity(old_signature.chunk_count) };
    let mut left = 0;

    loop {
        match find_reused_chunk::<R, S>(&old_signature, &new_content[left..]) {
            Some((bytes_until_reused, chunk_number, reused_strong_hash)) => {
                if bytes_until_reused > 0 {
                    delta.tokens.push(Added(&new_content[left..left + bytes_until_reused]));
                    left += bytes_until_reused;
                }
                delta.tokens.push(Reused(chunk_number, reused_strong_hash));
                left += old_signature.chunk_size;
            }
            None => {
                // couldn't find a single match until the end of the new content - finish up the delta
                delta.tokens.push(Added(&new_content[left..]));
                return Ok(delta);
            }
        }
    }
}

fn find_reused_chunk<R, S>(
    old_signature: &crate::Signature<R::ChecksumType, S::HashType>,
    new_content: &[u8],
) -> Option<(usize, chunk_number, S::HashType)> where
    R: RollingChecksum,
    <R as RollingChecksum>::ChecksumType: Eq + Hash,
    S: StrongHash,
{
    if old_signature.chunk_size > new_content.len() {
        // there isn't a whole chunk in the new content
        return None;
    }

    let mut rolling_checksum = R::new(&new_content[..old_signature.chunk_size]);
    let mut chunk_start = 0;

    loop {
        let checksum = rolling_checksum.checksum();

        if let Some(strong_hashes) = old_signature.quick_query(&checksum) {
            let hash = S::hash(&new_content[chunk_start..chunk_start + old_signature.chunk_size]);

            for (signature_hash, chunk_number) in strong_hashes {
                let signature_hash = *signature_hash;
                if signature_hash == hash {
                    return Some((chunk_start, *chunk_number, signature_hash));
                }
            }
        }

        if chunk_start + old_signature.chunk_size >= new_content.len() {
            break;
        }
        rolling_checksum.roll_window(
            new_content[chunk_start],
            new_content[chunk_start + old_signature.chunk_size]
        );
        chunk_start += 1;
    }

    return None;
}


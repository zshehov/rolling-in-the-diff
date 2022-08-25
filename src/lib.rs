use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

pub mod signature_generation;
pub mod delta_generation;

pub mod rolling_checksum;
pub mod strong_hash;


type ChunkNumber = u64;

pub trait StrongHash {
    type HashType: PartialEq + Debug + Copy;

    fn hash(data: &[u8]) -> Self::HashType;
}

pub struct Signature<W, S> where
    W: Eq + Hash + PartialEq,
    S: PartialEq + Copy,
{
    checksum_to_hashes: HashMap<W, Vec<(S, ChunkNumber)>>,
    chunk_size: usize,
    chunk_count: usize,
}

impl<W, S> Signature<W, S> where
    W: Eq + Hash + PartialEq,
    S: PartialEq + Copy,
{
    fn quick_query(&self, weak_checksum: &W) -> Option<&Vec<(S, ChunkNumber)>> {
        self.checksum_to_hashes.get(weak_checksum)
    }
}

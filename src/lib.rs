use std::collections::HashMap;
use std::hash::Hash;

pub mod signature_generation;
pub mod delta_generation;


type ChunkNumber = u64;

// the big question is here:
// How big can Signature get?
// It includes u32 and hash digest per chunk, so once a max file size is known, it will be apparent
// if just storing everything in a HashMap is possible. There is no obvious alternative at this point
// as there are these requirements on the struct:
// - Quick lookup by checksum
// - No FS operations are permitted during the usage of the Signature - meaning no loading/unloading
// from/to FS is possible
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

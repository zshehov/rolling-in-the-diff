use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::io::Write;

use log::debug;

use crate::delta_generation::{Delta, DeltaToken};
use crate::strong_hash::StrongHash;

type Result<T, S> = std::result::Result<T, PatchError<S>>;

#[derive(Debug)]
pub enum PatchError<S: Debug> {
    VersionMismatch { delta_ver: u64, old_content_ver: u64 },
    ChunkOutOfBound { chunk_num: u64, chunk_size: u64, old_content_len: u64 },
    ChunkHashMismatch { chunk_num: u64, old_content_hash: S, new_hash: S },
    OutputFailure(std::io::Error),
}

impl<S: Debug> Display for PatchError<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl <S: Debug> Error for PatchError<S> {

}

pub fn patch<S, W>(old_content: &[u8], delta: Delta<S::HashType>, out: &mut W) -> Result<(), S::HashType> where
    S: StrongHash,
    W: Write,
{
    if delta.base_content_version != 2 /*old file version*/ {}

    for token in delta.tokens {
        match token {
            DeltaToken::Reused(chunk_number, hash) => {
                match old_content.chunks(delta.chunk_size as usize).nth(chunk_number as usize) {
                    None => {
                        todo!("invalid case");
                    }
                    Some(chunk) => {
                        let actual_hash = S::hash(chunk);
                        if actual_hash == hash {
                            out.write_all(chunk).map_err(|e| PatchError::OutputFailure(e))?
                        } else {
                            return Err(PatchError::ChunkHashMismatch {
                                chunk_num: chunk_number,
                                old_content_hash: actual_hash,
                                new_hash: hash,
                            });
                        }
                    }
                }
            }
            DeltaToken::Added(bytes) => {
                out.write_all(bytes).map_err(|e| PatchError::OutputFailure(e))?
            }
            DeltaToken::Removed(chunk_number) => {
                debug!("chunk {} removed", chunk_number);
            }
        }
    }
    return Ok(());
}
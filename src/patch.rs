use std::fmt::{Debug};
use std::io::Write;

use log::debug;
use thiserror::Error;

use crate::delta_generation::{Delta, DeltaToken};
use crate::strong_hash::StrongHash;

pub fn patch<S, W>(
    old_content: &[u8],
    delta: Delta<S::HashType>,
    out: &mut W,
) -> Result<(), PatchError>
    where
        S: StrongHash,
        W: Write,
{
    for token in delta.tokens {
        match token {
            DeltaToken::Reused(chunk_number, hash) => {
                let chunk = old_content
                    .chunks(delta.chunk_size as usize)
                    .nth(chunk_number as usize)
                    .ok_or(PatchError::ChunkOutOfBound {
                        chunk_num: chunk_number,
                        chunk_size: delta.chunk_size,
                        old_content_len: old_content.len() as u64,
                    })?;

                if S::hash(chunk) != hash {
                    return Err(PatchError::ChunkHashMismatch {
                        chunk_num: chunk_number
                    });
                }

                out.write_all(chunk)?;
            }
            DeltaToken::Added(bytes) => {
                out.write_all(bytes)?;
            }
            DeltaToken::Removed(chunk_number) => {
                debug!("chunk {} removed", chunk_number);
            }
        }
    }
    return Ok(());
}

#[derive(Error, Debug)]
pub enum PatchError {
    #[error("chunk {chunk_num} is out of bound: {chunk_size} {old_content_len}")]
    ChunkOutOfBound {
        chunk_num: u64,
        chunk_size: u64,
        old_content_len: u64,
    },
    #[error("hash mismatch on chunk {chunk_num}")]
    ChunkHashMismatch {
        chunk_num: u64,
    },
    #[error("output error")]
    OutputFailure(#[from] std::io::Error),
}

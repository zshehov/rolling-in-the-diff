use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::io::Write;

use log::debug;
use serde::ser::StdError;

use crate::delta_generation::{Delta, DeltaToken};
use crate::strong_hash::StrongHash;

#[derive(Debug)]
pub enum PatchError<S: Debug> {
    ChunkOutOfBound { chunk_num: u64, chunk_size: u64, old_content_len: u64 },
    ChunkHashMismatch { chunk_num: u64, old_content_hash: S, new_hash: S },
    OutputFailure(std::io::Error),
}

pub fn patch<S, W>(old_content: &[u8], delta: Delta<S::HashType>, out: &mut W) where
    S: StrongHash,
    W: Write,
{
    let version = crate::VERSION.unwrap_or(crate::DEFAULT_VERSION);
    if delta.version != version {
        todo!("nicer error handling: {} {}",
              delta.version.to_string(),
              version.to_string());
    }

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
                            out.write_all(chunk).unwrap();
                        } else {
                            todo!("implement nicer error handling: {}", PatchError::ChunkHashMismatch {
                                chunk_num: chunk_number,
                                old_content_hash: actual_hash,
                                new_hash: hash,
                            }.to_string());
                        }
                    }
                }
            }
            DeltaToken::Added(bytes) => {
                out.write_all(bytes).unwrap();
            }
            DeltaToken::Removed(chunk_number) => {
                debug!("chunk {} removed", chunk_number);
            }
        }
    }
}

impl<S: Debug> Display for PatchError<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchError::ChunkOutOfBound { chunk_num, chunk_size, old_content_len } => {
                write!(f, "chunk out of bound for chunk {}: {} {}", chunk_num, chunk_size, old_content_len)
            }
            PatchError::ChunkHashMismatch { chunk_num, old_content_hash, new_hash } => {
                write!(f, "hash mismatch on chunk {}: {:?} {:?}", chunk_num, old_content_hash, new_hash)
            }
            PatchError::OutputFailure(..) => {
                write!(f, "failed on io")
            }
        }
    }
}

impl<S: Debug> Error for PatchError<S> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            PatchError::ChunkOutOfBound { .. } => { None }
            PatchError::ChunkHashMismatch { .. } => { None }
            PatchError::OutputFailure(err) => { Some(err) }
        }
    }
}

impl<S: Debug + Clone> From<std::io::Error> for PatchError<S> {
    fn from(err: std::io::Error) -> Self {
        PatchError::OutputFailure(err)
    }
}

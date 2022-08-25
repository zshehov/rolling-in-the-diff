use std::error;
use std::fmt::Debug;
use std::io::Read;
use std::marker::PhantomData;

mod adler32;

#[derive(Debug)]
pub enum Error {
    NoInput,
    FailedRead(std::io::Error),
}

pub trait RollingChecksum<T: Read>: Sized {
    type ChecksumType;

    // returns a checksum on the initial chunk_size chunk (chunk's length might be <= chunk_size)
    // and gets the stateful rolling checksum entity ready for rolling
    fn Create(input: T, chunk_size: usize) -> Result<(Self::ChecksumType, Self), Error>;
    // rolls the checksum sliding window forward one byte
    fn RollByte(&mut self) -> Option<Self::ChecksumType>;
}

pub trait StrongHash {
    type HashType: PartialEq + Debug;

    fn Hash(data: &[u8]) -> Self::HashType;
}

enum DeltaToken<'a, S> where
    S: /*Decode + Encode + */ PartialEq + Debug,
{
    Reused(
        u64 /* chunk number in old file */,
        S /* strong hash over the content for the patch operation to use*/,
    ),
    Added(&'a [u8] /* new data */),
}

pub struct Delta<'a, S> where
    S: /*Decode + Encode + */ PartialEq + Debug,
{
    tokens: Vec<DeltaToken<'a, S>>,
}

pub struct Generator<I, R, S> where
    I: Read,
// binding "I" to the RollingChecksum trait here isn't enough for the compiler
// to deduce variance rules for "I" (as it's not guaranteed that trait implementors will have
// a field of "type I"), forcing a PhantomData field
    R: RollingChecksum<I>,
    S: StrongHash,
{
    rolling_checksum: R,
    strong_hash: S,
    input_type: PhantomData<*const I>,
}

impl<I, R, S> Generator<I, R, S> where
    R: RollingChecksum<I>,
    S: StrongHash,
    I: Read,
{
    pub fn new(rolling_checksum: R, strong_hash: S) -> Self {
        Self { rolling_checksum, strong_hash, input_type: Default::default() }
    }

    pub fn generate_delta<'a>() -> Delta<'a, S::HashType> {
        todo!()
        // go through the new file with rolling hash adler32 <- TODO: add adler32 rolling hash implementation
        // check if there is a match with anything from the signature:
        //  - if yes: check if there is a strong hash match
        //      - if yes: add a DeltaToken::Reused entry,
        //          - if there was move 1 chunk forward
        //      - if no: roll one more byte
        //  - if no: roll one more byte
    }
}
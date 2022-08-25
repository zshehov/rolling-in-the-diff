pub mod signature_generator {
    use std::collections::HashMap;
    use std::fmt::Debug;
    use std::io::Read;

    pub trait WeakChecksum {
        type ChecksumType: PartialEq + Debug;

        fn Checksum(data: &[u8]) -> Self::ChecksumType;
    }

    pub trait StrongHash {
        type HashType: PartialEq + Debug;

        fn Hash(data: &[u8]) -> Self::HashType;
    }

    // TODO: implement marshaling to binary
    // probably use https://github.com/bincode-org/bincode
    // #[derive(Encode, Decode, PartialEq, Debug)]
    pub struct Signature<W, S> where
        W: /*Decode + Encode + */ PartialEq + Debug,
        S: /*Decode + Encode + */ PartialEq + Debug,
    {
        version: u8,
        chunk_size: u32,
        // ordered by sequence in the file
        chunks: Vec<(W, S)>,
    }

    pub struct Generator<W, S> where
        W: WeakChecksum,
        S: StrongHash,
    {
        checksum: W,
        strong_hash: S,
    }

    impl<W, S> Generator<W, S> where
        W: WeakChecksum,
        S: StrongHash,
    {
        pub fn new(checksum: W, strong_hash: S) -> Self {
            Self { checksum, strong_hash }
        }

        // TODO: maybe return an iterator with signature entries? What would be the pros and cons
        pub fn generate_signature() -> Signature<W::ChecksumType, S::HashType> {
            // Go through input split in chunks
            // calculate the adler32 checksum <- TODO: add adler32 checksum implementation
            // calculate the strong hash <- TODO: add a strong hash implementation
            // append to the signature chunks
            todo!()
        }
    }
}

pub mod delta_generator {
    use std::fmt::Debug;
    use std::io::Read;

    pub trait RollingChecksum {
        type ChecksumType;
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

    pub struct Generator<R, S> where
        R: RollingChecksum,
        S: StrongHash,
    {
        rolling_checksum: R,
        strong_hash: S,
    }

    impl<R, S> Generator<R, S> where
        R: RollingChecksum,
        S: StrongHash,
    {
        pub fn new(rolling_checksum: R, strong_hash: S) -> Self {
            Self { rolling_checksum, strong_hash }
        }

        pub fn generate_delta() -> Delta<S::HashType> {
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
}

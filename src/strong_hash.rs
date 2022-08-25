use std::fmt::Debug;

pub(crate) mod md5;

pub trait StrongHash {
    type HashType: PartialEq + Debug + Copy;

    fn hash(data: &[u8]) -> Self::HashType;
}


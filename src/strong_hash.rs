use std::fmt::Debug;

pub mod md5;

pub trait StrongHash {
    type HashType: Eq + PartialEq + Debug + Copy;

    fn hash(data: &[u8]) -> Self::HashType;
}

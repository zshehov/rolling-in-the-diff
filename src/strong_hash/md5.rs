use crate::strong_hash::StrongHash;

pub struct Md5Sum {}

impl StrongHash for Md5Sum {
    type HashType = [u8; 16];

    fn hash(data: &[u8]) -> Self::HashType {
        return md5::compute(data).into();
    }
}


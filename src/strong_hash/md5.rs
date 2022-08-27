use serde::{Deserialize, Serialize};

use crate::strong_hash::StrongHash;

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct Md5Sum {}

impl StrongHash for Md5Sum {
    type HashType = [u8; 16];

    fn hash(data: &[u8]) -> Self::HashType {
        md5::compute(data).into()
    }
}


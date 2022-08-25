pub mod rolling_adler32;

pub trait RollingChecksum {
    type ChecksumType;

    fn new(initial_data: &[u8]) -> Self;
    fn checksum(&self) -> Self::ChecksumType;

    fn push_byte(&mut self, new_byte: u8);
    fn pop_byte(&mut self, old_byte: u8, bytes_ago: usize);
}

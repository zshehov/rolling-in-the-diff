use std::collections::VecDeque;
use std::io::Read;

use crate::delta_generator::RollingChecksum;

struct RollingAdler32<T> where
    T: Read,
{
    actual_adler32: adler32::RollingAdler32,
    ring_bytes: VecDeque<u8>,
    input: T,
    chunk_size: usize,
}

impl<T> RollingChecksum<T> for RollingAdler32<T>
// TODO: why is this "where" required?
    where T: Read,
{
    type ChecksumType = u32;

    fn Create(input: T, chunk_size: usize) -> (Option<Self::ChecksumType>, Self) {
        return (
            Some(123),
            RollingAdler32 {
                // TODO: actually create the RollingAdler32 with a buffer read from the input
                actual_adler32: adler32::RollingAdler32::from_buffer(),
                // TODO: actually create the VecDeque with the first chunk bytes
                ring_bytes: VecDeque::with_capacity(chunk_size),
                input,
                chunk_size,
            },
        );
    }

    fn RollByte(&mut self) -> Option<Self::ChecksumType> {
        let mut single_byte = [0, 1];
        return match self.ring_bytes.pop_front() {
            Some(oldest_byte) => {
                self.actual_adler32.remove(self.ring_bytes.len() + 1, oldest_byte);

                match self.input.read(&mut single_byte) {
                    Ok(1) => {
                        self.ring_bytes.push_back(single_byte[0]);
                        self.actual_adler32.update(single_byte[0]);
                    }
                    Ok(_) => {
                        // when the reader is empty there are some ring_bytes remaining that have to be drained
                    }
                    Err(_) => {
                        None
                    }
                };
                Some(self.actual_adler32.hash())
            }
            None => {
                None
            }
        };
    }
}
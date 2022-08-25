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

    fn Create(mut input: T, chunk_size: usize) -> (Option<Self::ChecksumType>, Self) {
        let mut buff = vec![0; chunk_size];
        match input.read(&mut buff) {
            Ok(read_bytes) => {
                if read_bytes < chunk_size {
                    buff.truncate(read_bytes);
                }
                let checksum = adler32::RollingAdler32::from_buffer(&buff).hash();
                return (
                    Some(checksum),
                    RollingAdler32 {
                        actual_adler32: adler32::RollingAdler32::from_value(checksum),
                        ring_bytes: VecDeque::from(buff),
                        input,
                        chunk_size,
                    },
                );
            }
            Err(_) => {
                todo!()
            }
        }
    }

    fn RollByte(&mut self) -> Option<Self::ChecksumType> {
        let mut single_byte = [0];
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
                        // TODO: log the error when logger is available - even better return Result
                        return None;
                    }
                };
                Some(self.actual_adler32.hash())
            }
            None => {
                return None;
            }
        };
    }
}
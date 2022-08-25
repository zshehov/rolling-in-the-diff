use super::RollingChecksum;

pub struct RollingAdler32 {
    actual: adler32::RollingAdler32,
}

impl RollingChecksum for RollingAdler32 {
    type ChecksumType = u32;

    fn new(initial_window: &[u8]) -> Self {
        RollingAdler32 {
            actual: adler32::RollingAdler32::from_buffer(initial_window),
        }
    }

    fn checksum(&self) -> Self::ChecksumType {
        self.actual.hash()
    }

    fn push_byte(&mut self, new_byte: u8) {
        self.actual.update(new_byte);
    }

    fn pop_byte(&mut self, old_byte: u8, bytes_ago: usize) {
        self.actual.remove(bytes_ago, old_byte);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_sliding_window() {
        let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let window_size = 3;

        let mut rolling_checksum = RollingAdler32::new(&data[..window_size]);

        let mut left = 0;
        for right in window_size..data.len() {
            assert_eq!(rolling_checksum.checksum(), adler32::adler32(&data[left..right]).unwrap());
            rolling_checksum.pop_byte(data[left], window_size);
            rolling_checksum.push_byte(data[right]);
            left += 1;
        }

        // slide the left part of the window until all the data is consumed
        while left < data.len() {
            assert_eq!(rolling_checksum.checksum(), adler32::adler32(&data[left..]).unwrap());
            rolling_checksum.pop_byte(data[left], data.len() - left);
            left += 1;
        }
    }
}

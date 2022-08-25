use crate::delta_generation::RollingChecksum;

struct RollingAdler32 {
    actual: adler32::RollingAdler32,
    window_size: usize,
}

impl RollingChecksum for RollingAdler32 {
    type ChecksumType = u32;

    fn new(initial_window: &[u8]) -> Self {
        return RollingAdler32 {
            actual: adler32::RollingAdler32::from_buffer(initial_window),
            window_size: initial_window.len(),
        };
    }

    fn checksum(&self) -> Self::ChecksumType {
        self.actual.hash()
    }

    fn slide_window(&mut self, old_byte: u8, new_byte: u8) {
        self.actual.remove(self.window_size, old_byte);
        self.actual.update(new_byte);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_sliding_window() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let mut rolling_checksum = RollingAdler32::new(&data[..3]);

        let mut left = 0;
        for right in 3..data.len() - 1 {
            assert_eq!(rolling_checksum.checksum(), adler32::adler32(&data[left..right]).unwrap());
            rolling_checksum.slide_window(data[left], data[right]);
            left += 1;
        }
    }
}

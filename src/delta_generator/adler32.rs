use std::collections::VecDeque;
use std::io::Read;

use crate::delta_generator::{Error, RollingChecksum};
use crate::delta_generator::Error::NoInput;

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

    fn roll_window(&mut self, old_byte: u8, new_byte: u8) {
        self.actual.remove(self.window_size, old_byte);
        self.actual.update(new_byte);
    }
}
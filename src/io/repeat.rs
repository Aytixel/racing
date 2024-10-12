use std::io;

use super::AsyncRead;

#[derive(Debug)]
pub struct Repeat {
    pub(super) byte: u8,
}

impl AsyncRead for Repeat {
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        for byte in buf.iter_mut() {
            *byte = self.byte;
        }

        Ok(buf.len())
    }
}

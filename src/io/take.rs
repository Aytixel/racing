use std::{fmt, io};

use super::AsyncRead;

pub struct Take<T> {
    pub(super) reader: T,
    pub(super) limit: u64,
    pub(super) total: u64,
}

impl<T> Take<T> {
    pub(super) fn new(reader: T, limit: u64) -> Self {
        Self {
            reader,
            limit,
            total: 0,
        }
    }

    pub fn limit(&self) -> u64 {
        self.limit
    }

    pub fn set_limit(&mut self, limit: u64) {
        self.limit = limit;
    }

    pub fn into_inner(self) -> T {
        self.reader
    }

    pub fn get_ref(&self) -> &T {
        &self.reader
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.reader
    }
}

impl<T: fmt::Debug> fmt::Debug for Take<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Take")
            .field("reader", &self.reader)
            .field("limit", &self.limit)
            .finish()
    }
}

impl<T: AsyncRead> AsyncRead for Take<T> {
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self
            .reader
            .read(&mut buf[..(self.limit - self.total) as usize])
            .await
        {
            Ok(length) => {
                self.total += length as u64;
                Ok(length)
            }
            Err(error) => Err(error),
        }
    }
}

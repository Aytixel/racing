use std::{fmt, io};

use super::AsyncRead;

pub struct Chain<T, U> {
    pub(super) reader: T,
    pub(super) next: U,
    pub(super) first: bool,
}

impl<T, U> Chain<T, U> {
    pub(super) fn new(reader: T, next: U) -> Self {
        Self {
            reader,
            next,
            first: true,
        }
    }

    pub fn into_inner(self) -> (T, U) {
        (self.reader, self.next)
    }

    pub fn get_ref(&self) -> (&T, &U) {
        (&self.reader, &self.next)
    }

    pub fn get_mut(&mut self) -> (&mut T, &mut U) {
        (&mut self.reader, &mut self.next)
    }
}

impl<T: fmt::Debug, U: fmt::Debug> fmt::Debug for Chain<T, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Chain")
            .field("reader", &self.reader)
            .field("next", &self.next)
            .finish()
    }
}

impl<T: AsyncRead, U: AsyncRead> AsyncRead for Chain<T, U> {
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.first {
            match self.reader.read(buf).await {
                Ok(0) => {
                    self.first = false;

                    self.next.read(buf).await
                }
                Ok(length) => Ok(length),
                Err(error) => Err(error),
            }
        } else {
            self.next.read(buf).await
        }
    }
}

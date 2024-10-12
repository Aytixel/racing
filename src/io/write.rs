use std::{fmt::Arguments, future::Future, io};

use super::INIT_BUFFER_SIZE;

pub trait AsyncWrite {
    fn write(&mut self, buf: &[u8]) -> impl Future<Output = io::Result<usize>>;

    fn flush(&mut self) -> impl Future<Output = io::Result<()>>;

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> impl Future<Output = io::Result<()>> {
        async {
            let buffer =
                bufs.iter()
                    .fold(Vec::with_capacity(INIT_BUFFER_SIZE), |mut buffer, buf| {
                        buffer.extend_from_slice(buf);
                        buffer
                    });

            self.write_all(&buffer).await
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> impl Future<Output = io::Result<()>> {
        async {
            let mut length = 0;

            while length != buf.len() {
                length += self.write(&buf[length..]).await?;
            }

            Ok(())
        }
    }

    fn write_fmt(&mut self, fmt: Arguments<'_>) -> impl Future<Output = io::Result<()>> {
        async move { self.write_all(fmt.to_string().as_bytes()).await }
    }

    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
}

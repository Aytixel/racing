use std::{future::Future, io};

use super::{Chain, Take, INIT_BUFFER_SIZE};

pub trait AsyncRead {
    fn read(&mut self, buf: &mut [u8]) -> impl Future<Output = io::Result<usize>>;

    fn read_vectored(
        &mut self,
        bufs: &mut [io::IoSliceMut<'_>],
    ) -> impl Future<Output = io::Result<()>> {
        async {
            let mut buffer = Vec::with_capacity(
                bufs.iter()
                    .fold(0, |accumulator, buf| accumulator + buf.len()),
            );

            self.read_exact(&mut buffer).await?;

            bufs.iter_mut().fold(0, |accumulator, buf| {
                buf.clone_from_slice(&buffer[accumulator..]);
                accumulator + buf.len()
            });

            Ok(())
        }
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> impl Future<Output = io::Result<usize>> {
        async {
            let mut buffer = [0u8; INIT_BUFFER_SIZE];
            let mut total = 0;

            loop {
                match self.read(&mut buffer).await {
                    Ok(0) => break Ok(total),
                    Ok(length) => {
                        total += length;
                        buf.extend_from_slice(&buffer[..length]);
                    }
                    Err(error) => {
                        break match error.kind() {
                            io::ErrorKind::Interrupted => Ok(total),
                            _ => Err(error),
                        }
                    }
                }
            }
        }
    }

    fn read_to_string(&mut self, buf: &mut String) -> impl Future<Output = io::Result<usize>> {
        async {
            let mut buffer = Vec::with_capacity(INIT_BUFFER_SIZE);

            match self.read_to_end(&mut buffer).await {
                Ok(length) => {
                    *buf += &String::from_utf8(buffer)
                        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

                    Ok(length)
                }
                Err(error) => Err(error),
            }
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> impl Future<Output = io::Result<()>> {
        async {
            let mut total = 0;

            loop {
                match self.read(&mut buf[total..]).await {
                    Ok(0) => {
                        break if total != buf.len() {
                            Err(io::Error::new(io::ErrorKind::UnexpectedEof, ""))
                        } else {
                            Ok(())
                        }
                    }
                    Ok(length) => total += length,
                    Err(error) => {
                        break match error.kind() {
                            io::ErrorKind::Interrupted => {
                                if total != buf.len() {
                                    Err(io::Error::new(io::ErrorKind::UnexpectedEof, error))
                                } else {
                                    Ok(())
                                }
                            }
                            _ => Err(error),
                        }
                    }
                }
            }
        }
    }

    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }

    fn chain<R: AsyncRead>(self, next: R) -> impl Future<Output = Chain<Self, R>>
    where
        Self: Sized,
    {
        async { Chain::new(self, next) }
    }

    fn take(self, limit: u64) -> impl Future<Output = Take<Self>>
    where
        Self: Sized,
    {
        async move { Take::new(self, limit) }
    }
}

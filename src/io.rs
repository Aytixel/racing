use std::io::{ErrorKind, Result};

mod chain;
mod empty;
mod read;
mod repeat;
mod sink;
mod take;
mod write;

pub use chain::*;
pub use empty::*;
pub use read::*;
pub use repeat::*;
pub use sink::*;
pub use take::*;
pub use write::*;

pub mod prelude {
    pub use super::{AsyncRead, AsyncWrite};
}

pub(self) const INIT_BUFFER_SIZE: usize = 4096;

pub async fn copy<R, W>(reader: &mut R, writer: &mut W) -> Result<u64>
where
    R: AsyncRead + ?Sized,
    W: AsyncWrite + ?Sized,
{
    let mut buffer = [0u8; INIT_BUFFER_SIZE];
    let mut length = 0;

    loop {
        match reader.read(&mut buffer).await {
            Ok(0) => return Ok(length as u64),
            Ok(length_) => {
                length += length_;

                let mut length = 0;
                let buffer = &buffer[..length_];

                while length != buffer.len() {
                    match writer.write(&buffer[length..]).await {
                        Ok(length_) => length += length_,
                        Err(error) => match error.kind() {
                            ErrorKind::Interrupted => {}
                            _ => return Err(error),
                        },
                    }
                }
            }
            Err(error) => match error.kind() {
                ErrorKind::Interrupted => {}
                _ => return Err(error),
            },
        }
    }
}

pub const fn empty() -> Empty {
    Empty
}

pub async fn read_to_string<R: AsyncRead>(mut reader: R) -> Result<String> {
    let mut buffer = String::new();

    reader.read_to_string(&mut buffer).await?;

    Ok(buffer)
}

pub const fn repeat(byte: u8) -> Repeat {
    Repeat { byte }
}

pub const fn sink() -> Sink {
    Sink
}

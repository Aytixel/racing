use std::io;

use super::{AsyncRead, AsyncWrite};

#[derive(Clone, Copy, Debug, Default)]
pub struct Empty;

impl AsyncRead for Empty {
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        for byte in buf.iter_mut() {
            *byte = 0;
        }

        Ok(buf.len())
    }
}

impl AsyncWrite for &Empty {
    async fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    async fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl AsyncWrite for Empty {
    async fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&*self).write(buf).await
    }

    async fn flush(&mut self) -> io::Result<()> {
        (&*self).flush().await
    }
}

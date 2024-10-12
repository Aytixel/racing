use std::io;

use super::AsyncWrite;

#[derive(Clone, Copy, Debug, Default)]
pub struct Sink;

impl AsyncWrite for &Sink {
    async fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    async fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl AsyncWrite for Sink {
    async fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&*self).write(buf).await
    }

    async fn flush(&mut self) -> io::Result<()> {
        (&*self).flush().await
    }
}

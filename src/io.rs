use std::{
    fmt::{self, Arguments, Debug},
    future::Future,
    io::{Error, ErrorKind, IoSlice, IoSliceMut, Result},
};

const INIT_BUFFER_SIZE: usize = 4096;

pub struct Chain<T, U> {
    reader: T,
    next: U,
    first: bool,
}

impl<T, U> Chain<T, U> {
    fn new(reader: T, next: U) -> Self {
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

impl<T: Debug, U: Debug> Debug for Chain<T, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Chain")
            .field("reader", &self.reader)
            .field("next", &self.next)
            .finish()
    }
}

impl<T: AsyncRead, U: AsyncRead> AsyncRead for Chain<T, U> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
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

pub struct Take<T> {
    reader: T,
    limit: u64,
    total: u64,
}

impl<T> Take<T> {
    fn new(reader: T, limit: u64) -> Self {
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

impl<T: Debug> Debug for Take<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Take")
            .field("reader", &self.reader)
            .field("limit", &self.limit)
            .finish()
    }
}

impl<T: AsyncRead> AsyncRead for Take<T> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
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

pub trait AsyncRead {
    fn read(&mut self, buf: &mut [u8]) -> impl Future<Output = Result<usize>>;

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> impl Future<Output = Result<()>> {
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

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> impl Future<Output = Result<usize>> {
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
                            ErrorKind::Interrupted => Ok(total),
                            _ => Err(error),
                        }
                    }
                }
            }
        }
    }

    fn read_to_string(&mut self, buf: &mut String) -> impl Future<Output = Result<usize>> {
        async {
            let mut buffer = Vec::with_capacity(INIT_BUFFER_SIZE);

            match self.read_to_end(&mut buffer).await {
                Ok(length) => {
                    *buf += &String::from_utf8(buffer)
                        .map_err(|error| Error::new(ErrorKind::InvalidData, error))?;

                    Ok(length)
                }
                Err(error) => Err(error),
            }
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> impl Future<Output = Result<()>> {
        async {
            let mut total = 0;

            loop {
                match self.read(&mut buf[total..]).await {
                    Ok(0) => {
                        break if total != buf.len() {
                            Err(Error::new(ErrorKind::UnexpectedEof, ""))
                        } else {
                            Ok(())
                        }
                    }
                    Ok(length) => total += length,
                    Err(error) => {
                        break match error.kind() {
                            ErrorKind::Interrupted => {
                                if total != buf.len() {
                                    Err(Error::new(ErrorKind::UnexpectedEof, error))
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

pub trait AsyncWrite {
    fn write(&mut self, buf: &[u8]) -> impl Future<Output = Result<usize>>;

    fn flush(&mut self) -> impl Future<Output = Result<()>>;

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> impl Future<Output = Result<()>> {
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

    fn write_all(&mut self, buf: &[u8]) -> impl Future<Output = Result<()>> {
        async {
            let mut length = 0;

            while length != buf.len() {
                length += self.write(&buf[length..]).await?;
            }

            Ok(())
        }
    }

    fn write_fmt(&mut self, fmt: Arguments<'_>) -> impl Future<Output = Result<()>> {
        async move { self.write_all(fmt.to_string().as_bytes()).await }
    }

    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
}

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

#[derive(Clone, Copy, Debug, Default)]
pub struct Empty;

impl AsyncRead for Empty {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        for byte in buf.iter_mut() {
            *byte = 0;
        }

        Ok(buf.len())
    }
}

impl AsyncWrite for &Empty {
    async fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl AsyncWrite for Empty {
    async fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (&*self).write(buf).await
    }

    async fn flush(&mut self) -> Result<()> {
        (&*self).flush().await
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

#[derive(Debug)]
pub struct Repeat {
    byte: u8,
}

impl AsyncRead for Repeat {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        for byte in buf.iter_mut() {
            *byte = self.byte;
        }

        Ok(buf.len())
    }
}

pub const fn repeat(byte: u8) -> Repeat {
    Repeat { byte }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Sink;

impl AsyncWrite for &Sink {
    async fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl AsyncWrite for Sink {
    async fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (&*self).write(buf).await
    }

    async fn flush(&mut self) -> Result<()> {
        (&*self).flush().await
    }
}

pub const fn sink() -> Sink {
    Sink
}

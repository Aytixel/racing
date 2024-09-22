use std::{
    collections::VecDeque,
    future::poll_fn,
    io::{Error, ErrorKind, Read, Result, Write},
    net::{self, Shutdown, SocketAddr, ToSocketAddrs},
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd},
    task::Poll,
    time::{Duration, Instant},
};

use crate::io::{AsyncRead, AsyncWrite};

#[derive(Debug)]
pub struct TcpStream(net::TcpStream);

impl TcpStream {
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> Result<TcpStream> {
        Self::connect_timeout(addr, Duration::from_secs(1)).await
    }

    pub async fn connect_timeout<A: ToSocketAddrs>(
        addr: A,
        max_timeout: Duration,
    ) -> Result<TcpStream> {
        let mut addresses: VecDeque<SocketAddr> = addr.to_socket_addrs()?.collect();
        let mut error = None;
        let mut timeout = Duration::from_millis(50);

        poll_fn(|_context| {
            if addresses.is_empty() {
                Poll::Ready(if let Some(error) = error.take() {
                    Err(error)
                } else {
                    Err(Error::new(
                        ErrorKind::AddrNotAvailable,
                        "No SocketAddr provided",
                    ))
                })
            } else {
                let address = addresses.pop_front().unwrap();

                match net::TcpStream::connect_timeout(&address, timeout) {
                    Ok(stream) => {
                        if let Err(error) = stream.set_nonblocking(true) {
                            Poll::Ready(Err(error))
                        } else {
                            Poll::Ready(Ok(TcpStream(stream)))
                        }
                    }
                    Err(error_) => {
                        if let ErrorKind::TimedOut = error_.kind() {
                            timeout = (timeout * 2).min(max_timeout);

                            addresses.push_back(address);
                        } else {
                            error = Some(error_);
                        }

                        Poll::Pending
                    }
                }
            }
        })
        .await
    }

    pub fn peer_addr(&self) -> Result<SocketAddr> {
        self.0.peer_addr()
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.0.local_addr()
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.0.shutdown(how)
    }

    pub fn try_clone(&self) -> Result<TcpStream> {
        Ok(TcpStream(self.0.try_clone()?))
    }

    pub fn set_read_timeout(&self, dur: Option<Duration>) -> Result<()> {
        self.0.set_read_timeout(dur)
    }

    pub fn set_write_timeout(&self, dur: Option<Duration>) -> Result<()> {
        self.0.set_write_timeout(dur)
    }

    pub fn read_timeout(&self) -> Result<Option<Duration>> {
        self.0.read_timeout()
    }

    pub fn write_timeout(&self) -> Result<Option<Duration>> {
        self.0.write_timeout()
    }

    pub async fn peek(&self, buf: &mut [u8]) -> Result<usize> {
        poll_fn(|_context| match self.0.peek(buf) {
            Ok(length) => Poll::Ready(Ok(length)),
            Err(error) => match error.kind() {
                ErrorKind::WouldBlock => Poll::Pending,
                _ => Poll::Ready(Err(error)),
            },
        })
        .await
    }

    pub fn set_nodelay(&self, nodelay: bool) -> Result<()> {
        self.0.set_nodelay(nodelay)
    }

    pub fn set_ttl(&self, ttl: u32) -> Result<()> {
        self.0.set_ttl(ttl)
    }

    pub fn ttl(&self) -> Result<u32> {
        self.0.ttl()
    }

    pub fn take_error(&self) -> Result<Option<Error>> {
        self.0.take_error()
    }
}

impl AsFd for TcpStream {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl AsRawFd for TcpStream {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl From<OwnedFd> for TcpStream {
    fn from(value: OwnedFd) -> Self {
        TcpStream(net::TcpStream::from(value))
    }
}

impl From<TcpStream> for OwnedFd {
    fn from(value: TcpStream) -> Self {
        OwnedFd::from(value.0)
    }
}

impl FromRawFd for TcpStream {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        TcpStream(net::TcpStream::from_raw_fd(fd))
    }
}

impl IntoRawFd for TcpStream {
    fn into_raw_fd(self) -> RawFd {
        self.0.into_raw_fd()
    }
}

impl AsyncRead for &TcpStream {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut stream = self.0.try_clone()?;

        if let Ok(Some(duration)) = self.read_timeout() {
            if duration.is_zero() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Timeout duration can't be zero",
                ));
            }

            let instant = Instant::now() + duration;

            poll_fn(|_context| {
                if instant.checked_duration_since(Instant::now()).is_none() {
                    return Poll::Ready(Err(Error::new(
                        ErrorKind::TimedOut,
                        "TcpStream timed out",
                    )));
                }

                match stream.read(buf) {
                    Ok(length) => Poll::Ready(Ok(length)),
                    Err(error) => match error.kind() {
                        ErrorKind::WouldBlock => Poll::Pending,
                        _ => Poll::Ready(Err(error)),
                    },
                }
            })
            .await
        } else {
            poll_fn(|_context| match stream.read(buf) {
                Ok(length) => Poll::Ready(Ok(length)),
                Err(error) => match error.kind() {
                    ErrorKind::WouldBlock => Poll::Pending,
                    _ => Poll::Ready(Err(error)),
                },
            })
            .await
        }
    }
}

impl AsyncRead for TcpStream {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (&*self).read(buf).await
    }
}

impl AsyncWrite for &TcpStream {
    async fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut stream = self.0.try_clone()?;

        if let Ok(Some(duration)) = self.write_timeout() {
            if duration.is_zero() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Timeout duration can't be zero",
                ));
            }

            let instant = Instant::now() + duration;

            poll_fn(|_context| {
                if instant.checked_duration_since(Instant::now()).is_none() {
                    return Poll::Ready(Err(Error::new(
                        ErrorKind::TimedOut,
                        "TcpStream timed out",
                    )));
                }

                match stream.write(buf) {
                    Ok(length) => Poll::Ready(Ok(length)),
                    Err(error) => match error.kind() {
                        ErrorKind::WouldBlock => Poll::Pending,
                        _ => Poll::Ready(Err(error)),
                    },
                }
            })
            .await
        } else {
            poll_fn(|_context| match stream.write(buf) {
                Ok(length) => Poll::Ready(Ok(length)),
                Err(error) => match error.kind() {
                    ErrorKind::WouldBlock => Poll::Pending,
                    _ => Poll::Ready(Err(error)),
                },
            })
            .await
        }
    }

    async fn flush(&mut self) -> Result<()> {
        let mut stream = self.0.try_clone()?;

        if let Ok(Some(duration)) = self.write_timeout() {
            if duration.is_zero() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Timeout duration can't be zero",
                ));
            }

            let instant = Instant::now() + duration;

            poll_fn(|_context| {
                if instant.checked_duration_since(Instant::now()).is_none() {
                    return Poll::Ready(Err(Error::new(
                        ErrorKind::TimedOut,
                        "TcpStream timed out",
                    )));
                }

                match stream.flush() {
                    Ok(()) => Poll::Ready(Ok(())),
                    Err(error) => match error.kind() {
                        ErrorKind::WouldBlock => Poll::Pending,
                        _ => Poll::Ready(Err(error)),
                    },
                }
            })
            .await
        } else {
            poll_fn(|_context| match stream.flush() {
                Ok(()) => Poll::Ready(Ok(())),
                Err(error) => match error.kind() {
                    ErrorKind::WouldBlock => Poll::Pending,
                    _ => Poll::Ready(Err(error)),
                },
            })
            .await
        }
    }
}

impl AsyncWrite for TcpStream {
    async fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (&*self).write(buf).await
    }

    async fn flush(&mut self) -> Result<()> {
        (&*self).flush().await
    }
}

use std::{
    future::poll_fn,
    io::{Error, ErrorKind, Result},
    net::{self, SocketAddr, ToSocketAddrs},
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd},
    task::Poll,
};

use super::TcpStream;

#[derive(Debug)]
pub struct TcpListener(net::TcpListener);

impl TcpListener {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> Result<TcpListener> {
        let listener = net::TcpListener::bind(addr)?;

        if let Err(error) = listener.set_nonblocking(true) {
            Err(error)
        } else {
            Ok(TcpListener(listener))
        }
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.0.local_addr()
    }

    pub fn try_clone(&self) -> Result<TcpListener> {
        Ok(TcpListener(self.0.try_clone()?))
    }

    pub async fn accept(&self) -> Result<(TcpStream, SocketAddr)> {
        poll_fn(|_context| match self.0.accept() {
            Ok((stream, address)) => Poll::Ready(Ok((TcpStream(stream), address))),
            Err(error) => match error.kind() {
                ErrorKind::WouldBlock => Poll::Pending,
                _ => Poll::Ready(Err(error)),
            },
        })
        .await
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

impl AsFd for TcpListener {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl AsRawFd for TcpListener {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl From<OwnedFd> for TcpListener {
    fn from(value: OwnedFd) -> Self {
        TcpListener(net::TcpListener::from(value))
    }
}

impl From<TcpListener> for OwnedFd {
    fn from(value: TcpListener) -> Self {
        OwnedFd::from(value.0)
    }
}

impl FromRawFd for TcpListener {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        TcpListener(net::TcpListener::from_raw_fd(fd))
    }
}

impl IntoRawFd for TcpListener {
    fn into_raw_fd(self) -> RawFd {
        self.0.into_raw_fd()
    }
}

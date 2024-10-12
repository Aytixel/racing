use std::{
    future::poll_fn,
    io::{Error, ErrorKind, Result},
    net::{self, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs},
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd},
    task::Poll,
    time::{Duration, Instant},
};

use crate::net::poll_net;

pub struct UdpSocket(net::UdpSocket);

impl UdpSocket {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> Result<UdpSocket> {
        let socket = net::UdpSocket::bind(addr)?;

        if let Err(error) = socket.set_nonblocking(true) {
            Err(error)
        } else {
            Ok(UdpSocket(socket))
        }
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        poll_net!(self.0, self.read_timeout(), UdpSocket::recv_from(buf))
    }

    pub async fn peek_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        poll_net!(self.0, self.read_timeout(), UdpSocket::peek_from(buf))
    }

    pub async fn send_to<A: ToSocketAddrs>(&self, buf: &[u8], addr: A) -> Result<usize> {
        let addrs: Vec<SocketAddr> = addr.to_socket_addrs()?.collect();

        poll_net!(
            self.0,
            self.write_timeout(),
            UdpSocket::send_to(buf, &addrs[..])
        )
    }

    pub fn peer_addr(&self) -> Result<SocketAddr> {
        self.0.peer_addr()
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.0.local_addr()
    }

    pub fn try_clone(&self) -> Result<UdpSocket> {
        Ok(UdpSocket(self.0.try_clone()?))
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

    pub fn set_broadcast(&self, broadcast: bool) -> Result<()> {
        self.0.set_broadcast(broadcast)
    }

    pub fn broadcast(&self) -> Result<bool> {
        self.0.broadcast()
    }

    pub fn set_multicast_loop_v4(&self, multicast_loop_v4: bool) -> Result<()> {
        self.0.set_multicast_loop_v4(multicast_loop_v4)
    }

    pub fn multicast_loop_v4(&self) -> Result<bool> {
        self.0.multicast_loop_v4()
    }

    pub fn set_multicast_ttl_v4(&self, multicast_ttl_v4: u32) -> Result<()> {
        self.0.set_multicast_ttl_v4(multicast_ttl_v4)
    }

    pub fn multicast_ttl_v4(&self) -> Result<u32> {
        self.0.multicast_ttl_v4()
    }

    pub fn set_multicast_loop_v6(&self, multicast_loop_v6: bool) -> Result<()> {
        self.0.set_multicast_loop_v6(multicast_loop_v6)
    }

    pub fn multicast_loop_v6(&self) -> Result<bool> {
        self.0.multicast_loop_v6()
    }

    pub fn set_ttl(&self, ttl: u32) -> Result<()> {
        self.0.set_ttl(ttl)
    }

    pub fn ttl(&self) -> Result<u32> {
        self.0.ttl()
    }

    pub fn join_multicast_v4(&self, multiaddr: &Ipv4Addr, interface: &Ipv4Addr) -> Result<()> {
        self.0.join_multicast_v4(multiaddr, interface)
    }

    pub fn join_multicast_v6(&self, multiaddr: &Ipv6Addr, interface: u32) -> Result<()> {
        self.0.join_multicast_v6(multiaddr, interface)
    }

    pub fn leave_multicast_v4(&self, multiaddr: &Ipv4Addr, interface: &Ipv4Addr) -> Result<()> {
        self.0.leave_multicast_v4(multiaddr, interface)
    }

    pub fn leave_multicast_v6(&self, multiaddr: &Ipv6Addr, interface: u32) -> Result<()> {
        self.0.leave_multicast_v6(multiaddr, interface)
    }

    pub fn take_error(&self) -> Result<Option<Error>> {
        self.0.take_error()
    }

    pub fn connect<A: ToSocketAddrs>(&self, addr: A) -> Result<()> {
        self.0.connect(addr)
    }

    pub async fn send(&self, buf: &[u8]) -> Result<usize> {
        poll_net!(self.0, self.write_timeout(), UdpSocket::send(buf))
    }

    pub async fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        poll_net!(self.0, self.read_timeout(), UdpSocket::recv(buf))
    }

    pub async fn peek(&self, buf: &mut [u8]) -> Result<usize> {
        poll_net!(self.0, self.read_timeout(), UdpSocket::peek(buf))
    }
}

impl AsFd for UdpSocket {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl AsRawFd for UdpSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl From<OwnedFd> for UdpSocket {
    fn from(value: OwnedFd) -> Self {
        UdpSocket(net::UdpSocket::from(value))
    }
}

impl From<UdpSocket> for OwnedFd {
    fn from(value: UdpSocket) -> Self {
        OwnedFd::from(value.0)
    }
}

impl FromRawFd for UdpSocket {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        UdpSocket(net::UdpSocket::from_raw_fd(fd))
    }
}

impl IntoRawFd for UdpSocket {
    fn into_raw_fd(self) -> RawFd {
        self.0.into_raw_fd()
    }
}

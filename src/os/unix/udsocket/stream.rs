use super::{
    c_wrappers,
    cmsg::{context::Collector, CmsgMutBuf, CmsgRef},
    util::{make_msghdr_r, make_msghdr_w},
    ToUdSocketPath, UdSocketPath,
};
use crate::{
    os::unix::{unixprelude::*, FdOps},
    TryClone,
};
use libc::{sockaddr_un, SOCK_STREAM};
use std::io::{self, IoSlice, IoSliceMut, Read, Write};
use to_method::To;

/// A Unix domain socket byte stream, obtained either from [`UdStreamListener`](super::UdStreamListener) or by
/// connecting to an existing server.
///
/// # Examples
///
/// ## Basic client
/// ```no_run
/// use interprocess::os::unix::udsocket::UdStream;
/// use std::io::prelude::*;
///
/// let mut conn = UdStream::connect("/tmp/example1.sock")?;
/// conn.write_all(b"Hello from client!")?;
/// let mut string_buffer = String::new();
/// conn.read_to_string(&mut string_buffer)?;
/// println!("Server answered: {}", string_buffer);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
// TODO update with comments and stuff
#[derive(Debug)]
pub struct UdStream(FdOps);
impl UdStream {
    /// Connects to a Unix domain socket server at the specified path.
    ///
    /// See [`ToUdSocketPath`] for an example of using various string types to specify socket paths.
    ///
    /// # System calls
    /// - `socket`
    /// - `connect`
    pub fn connect<'a>(path: impl ToUdSocketPath<'a>) -> io::Result<Self> {
        Self::_connect(path.to_socket_path()?, false)
    }
    #[cfg(feature = "tokio")]
    pub(crate) fn connect_nonblocking<'a>(path: impl ToUdSocketPath<'a>) -> io::Result<Self> {
        Self::_connect(path.to_socket_path()?, true)
    }
    fn _connect(path: UdSocketPath<'_>, nonblocking: bool) -> io::Result<Self> {
        let addr = path.try_to::<sockaddr_un>()?;

        let fd = c_wrappers::create_uds(SOCK_STREAM, nonblocking)?;
        unsafe {
            // SAFETY: addr is well-constructed
            c_wrappers::connect(fd.0.as_fd(), &addr)?;
        }

        Ok(Self(fd))
    }

    /// Receives both bytes and ancillary data from the socket stream. The first element of the return value
    /// represents the read amount of the former, while the second element represents that of the latter.
    ///
    /// # System calls
    /// - `recvmsg`
    #[inline]
    pub fn recv_ancillary<E: Collector>(
        &self,
        buf: &mut [u8],
        abuf: &mut CmsgMutBuf<'_, E>,
    ) -> io::Result<(usize, usize)> {
        self.recv_ancillary_vectored(&mut [IoSliceMut::new(buf)], abuf)
    }
    /// Receives bytes and ancillary data from the socket stream, making use of [scatter input] for the main data.
    /// The first element of the return value represents the read amount of the former, while the second element
    /// represents that of the latter.
    ///
    /// # System calls
    /// - `recvmsg`
    ///
    /// [scatter input]: https://en.wikipedia.org/wiki/Vectored_I/O " "
    #[inline]
    pub fn recv_ancillary_vectored<E: Collector>(
        &self,
        bufs: &mut [IoSliceMut<'_>],
        abuf: &mut CmsgMutBuf<'_, E>,
    ) -> io::Result<(usize, usize)> {
        let mut hdr = make_msghdr_r(bufs, abuf)?;
        let fd = self.as_fd();

        abuf.context_collector.pre_op_collect(fd);
        let bytes_read = unsafe {
            // SAFETY: make_msghdr_r is good at its job
            c_wrappers::recvmsg(fd, &mut hdr, 0)?
        };
        abuf.context_collector.post_op_collect(fd, hdr.msg_flags);

        Ok((bytes_read, hdr.msg_controllen as _))
    }

    /// Sends bytes and ancillary data into the socket stream.
    ///
    /// # System calls
    /// - `sendmsg`
    #[inline]
    pub fn send_ancillary(&self, buf: &[u8], abuf: CmsgRef<'_, '_>) -> io::Result<usize> {
        self.send_ancillary_vectored(&[IoSlice::new(buf)], abuf)
    }
    /// Sends bytes and ancillary data into the socket stream, making use of [gather output] for the main data.
    ///
    /// # System calls
    /// - `sendmsg`
    ///
    /// [gather output]: https://en.wikipedia.org/wiki/Vectored_I/O " "
    pub fn send_ancillary_vectored(&self, bufs: &[IoSlice<'_>], abuf: CmsgRef<'_, '_>) -> io::Result<usize> {
        let hdr = make_msghdr_w(bufs, abuf)?;
        unsafe {
            // SAFETY: make_msghdr_w is good at its job
            c_wrappers::sendmsg(self.as_fd(), &hdr, 0)
        }
    }
}

/// A list of used system calls is available.
impl Read for &UdStream {
    /// # System calls
    /// - `read`
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&self.0).read(buf)
    }
    /// # System calls
    /// - `readv`
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        (&self.0).read_vectored(bufs)
    }
}
/// A list of used system calls is available.
impl Read for UdStream {
    /// # System calls
    /// - `read`
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&*self).read(buf)
    }
    /// # System calls
    /// - `readv`
    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        (&*self).read_vectored(bufs)
    }
}
/// A list of used system calls is available.
impl Write for &UdStream {
    /// # System calls
    /// - `write`
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&self.0).write(buf)
    }
    /// # System calls
    /// - `writev`
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        (&self.0).write_vectored(bufs)
    }
    /// # System calls
    /// None performed.
    fn flush(&mut self) -> io::Result<()> {
        // You cannot flush a socket
        Ok(())
    }
}
/// A list of used system calls is available.
impl Write for UdStream {
    /// # System calls
    /// - `write`
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&*self).write(buf)
    }
    /// # System calls
    /// - `writev`
    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        (&*self).write_vectored(bufs)
    }
    /// # System calls
    /// None performed.
    fn flush(&mut self) -> io::Result<()> {
        // You cannot flush a socket
        Ok(())
    }
}

impl TryClone for UdStream {
    fn try_clone(&self) -> io::Result<Self> {
        self.0.try_clone().map(Self)
    }
}

impl AsFd for UdStream {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0 .0.as_fd()
    }
}
impl From<UdStream> for OwnedFd {
    #[inline]
    fn from(x: UdStream) -> Self {
        x.0 .0
    }
}
impl From<OwnedFd> for UdStream {
    #[inline]
    fn from(fd: OwnedFd) -> Self {
        UdStream(FdOps(fd))
    }
}

derive_raw!(unix: UdStream);

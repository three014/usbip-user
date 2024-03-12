pub mod detach;
pub mod attach {
    use usbip_core::buffer::Buffer;

    use crate::{net, protocol};

    fn query_import_device<S>(mut socket: S, bus_id: &str) -> bincode::Result<u16>
    where
        S: net::Send + net::Recv,
    {
        let request = net::OpCommon {
            version: net::VERSION,
            code: protocol::OP_REQ_IMPORT,
            status: usbip_core::net::Status::Success,
        };

        socket.send(&request)?;

        let request = net::OpImportRequest {
            bus_id: Buffer::try_from(bus_id.as_bytes()).unwrap(),
        };

        socket.send(&request)?;

        todo!()
    }
}
pub mod protocol {
    // Common header for all the kinds of PDUs.
    pub const OP_REQUEST: u16 = 0x80 << 8;
    pub const OP_REPLY: u16 = 0x00 << 8;

    // Import a remote USB device.
    pub const OP_IMPORT: u16 = 0x03;
    pub const OP_REQ_IMPORT: u16 = OP_REQUEST | OP_IMPORT;
    pub const OP_REP_IMPORT: u16 = OP_REPLY | OP_IMPORT;

    // Dummy code
    pub const OP_UNSPEC: u16 = 0x00;
    pub const _OP_REQ_UNSPEC: u16 = OP_UNSPEC;
    pub const _OP_REP_UNSPEC: u16 = OP_UNSPEC;

    // Retrieve the list of exported USB devices
    pub const OP_DEVLIST: u16 = 0x05;
    pub const OP_REQ_DEVLIST: u16 = OP_REQUEST | OP_DEVLIST;
    pub const OP_REP_DEVLIST: u16 = OP_REPLY | OP_DEVLIST;

    // Export a USB device to a remote host
    pub const OP_EXPORT: u16 = 0x06;
    pub const OP_REQ_EXPORT: u16 = OP_REQUEST | OP_EXPORT;
    pub const OP_REP_EXPORT: u16 = OP_REPLY | OP_EXPORT;
}

pub mod net {
    use bincode::Options;
    use serde::{de::DeserializeOwned, Deserialize, Serialize};
    use std::{io, os::fd::AsRawFd};
    use usbip_core::{buffer::Buffer, UsbDevice, SYSFS_BUS_ID_SIZE};

    pub use error::Error;

    use crate::protocol::OP_UNSPEC;

    mod error {
        use std::fmt;

        #[derive(Debug, Clone)]
        pub enum Error {
            VersionMismatch(u16),
            BusIdMismatch(Box<str>),
        }

        impl fmt::Display for Error {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    Error::VersionMismatch(bad_version) => write!(
                        f,
                        "version mismatch! Them: {}, Us: {}",
                        bad_version,
                        super::VERSION
                    ),
                    Error::BusIdMismatch(bus_id) => write!(f, "received different busid: {bus_id}"),
                }
            }
        }

        impl std::error::Error for Error {}
    }

    pub const VERSION: u16 = 273;

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct OpCommon {
        pub version: u16,
        pub code: u16,
        pub status: usbip_core::net::Status,
    }

    impl OpCommon {
        pub fn validate(&self, code: u16) -> Result<usbip_core::net::Status, Error> {
            if self.version != VERSION {
                Err(Error::VersionMismatch(self.version))
            } else if !matches!(code, OP_UNSPEC) && code != self.code {
                Ok(usbip_core::net::Status::Unexpected)
            } else {
                Ok(self.status)
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct OpImportRequest {
        pub bus_id: Buffer<SYSFS_BUS_ID_SIZE, i8>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct OpImportReply {
        pub udev: UsbDevice,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct OpDevlistReply {
        pub ndev: u32,
    }

    fn socket_set_keepalive(socket: &std::net::TcpStream, keepalive: bool) -> io::Result<()> {
        use libc::{c_int, c_void, socklen_t};

        let val = c_int::from(keepalive);
        let rc = unsafe {
            libc::setsockopt(
                socket.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_KEEPALIVE,
                std::ptr::addr_of!(val).cast::<c_void>(),
                socklen_t::try_from(std::mem::size_of::<c_int>()).unwrap(),
            )
        };
        if rc < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Opens a TCP connection to a remote host.
    /// It is not required to use this function to initiate
    /// the connection, as long as these socket options
    /// (or their equivalents) are set:
    /// - `NoDelay` is enabled (disables the Nagle algorithm)
    /// - `KeepAlive` is enabled
    pub fn connect<A>(host: A) -> io::Result<std::net::TcpStream>
    where
        A: std::net::ToSocketAddrs,
    {
        let socket = std::net::TcpStream::connect(host)?;
        socket.set_nodelay(true)?;
        socket_set_keepalive(&socket, true)?;
        Ok(socket)
    }

    fn bincode_options() -> impl bincode::Options {
        bincode::DefaultOptions::new()
            .with_no_limit()
            .with_big_endian()
            .with_fixint_encoding()
            .allow_trailing_bytes()
    }

    pub trait Send: io::Write {
        fn send<T>(&mut self, value: &T) -> bincode::Result<()>
        where
            T: Serialize + ?Sized,
        {
            bincode_options().serialize_into(self, value)
        }
    }

    pub trait Recv: io::Read {
        fn recv<T>(&mut self) -> bincode::Result<T>
        where
            T: DeserializeOwned,
        {
            bincode_options().deserialize_from(self)
        }
    }

    impl Recv for std::net::TcpStream {}
    impl Send for std::net::TcpStream {}
}

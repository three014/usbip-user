pub use detach::detach_port;

pub mod detach;
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
}

pub mod net {
    use serde::{Deserialize, Serialize, Serializer};
    use std::{io, os::fd::AsRawFd};
    use usbip_core::{buffer::Buffer, UsbDevice, SYSFS_BUS_ID_SIZE};

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    struct OpCommon {
        version: u16,
        code: u16,
        status: usbip_core::net::Status,
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
                &val as *const c_int as *const c_void,
                std::mem::size_of::<c_int>() as socklen_t,
            )
        };
        if rc < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn connect<A>(host: A) -> io::Result<std::net::TcpStream>
    where
        A: std::net::ToSocketAddrs,
    {
        let socket = std::net::TcpStream::connect(host)?;
        socket.set_nodelay(true)?;
        socket_set_keepalive(&socket, true)?;
        Ok(socket)
    }
}

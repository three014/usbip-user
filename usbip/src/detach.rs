use std::{error::Error as StdError, fs, path::PathBuf};

use usbip_core::{
    vhci,
    DeviceStatus,
};

pub use error::Error;

mod error {
    use std::fmt;

    #[derive(Debug, Clone, Copy)]
    pub enum Error {
        PortAlreadyDetached(u8),
        InvalidPort { requested: u8, num_ports: usize },
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Error::PortAlreadyDetached(requested) => {
                    write!(f, "Port {requested} is already detached!")
                }
                Error::InvalidPort {
                    requested,
                    num_ports,
                } => write!(f, "Invalid port {requested} > maxports ({num_ports})"),
            }
        }
    }

    impl std::error::Error for Error {}
}

/// Iterates through `idevs` to verify that
/// `port` is a valid port number.
/// 
/// # Error
/// This function returns an error if the requested
/// port was already detached or if the port number
/// was higher than the maximum number of ports on
/// this system.
fn validate(
    port: u8,
    mut idevs: impl ExactSizeIterator<Item = vhci::ImportedDevice>,
) -> Result<(), Error> {
    let num_ports = idevs.len();
    let idev = idevs
        .find(|idev| idev.port() == port)
        .ok_or(Error::InvalidPort {
            requested: port,
            num_ports,
        })?;
    if matches!(idev.status(), DeviceStatus::PortAvailable) {
        Err(Error::PortAlreadyDetached(port))
    } else {
        Ok(())
    }
}

/// Detaches a remote USB device from the system.
///
/// # Errors
/// This function can fail for these reasons below:
/// - `port` was already detached
/// - `port` was not a valid port number
/// - There was an error with the Vhci driver
/// (see `usbip_core::vhci::Driver::try_open`)
pub fn detach_port(port: u8) -> Result<(), Box<dyn StdError>> {
    let driver = vhci::Driver::try_open()?;

    let imported_devices = driver.imported_devices();
    validate(port, imported_devices)?;

    let path = PathBuf::from(format!("{}/port{}", vhci::STATE_PATH, port));
    let _ = fs::remove_file(path);
    let _ = fs::remove_dir(vhci::STATE_PATH);

    driver
        .try_detach_dev(port)
        .map_err(std::convert::Into::into)
}

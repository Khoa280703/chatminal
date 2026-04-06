use crate::config::validate_target_name;
use dynamic::{FromDynamic, ToDynamic};

#[derive(Default, Debug, Clone, FromDynamic, ToDynamic)]
pub struct SerialTarget {
    /// The name of this specific target. Must be unique amongst
    /// all target types in the configuration file.
    #[dynamic(validate = "validate_target_name")]
    pub name: String,

    /// Specifies the serial device name.
    /// On Windows systems this can be a name like `COM0`.
    /// On posix systems this will be something like `/dev/ttyUSB0`.
    /// If omitted, the name will be interpreted as the port.
    pub port: Option<String>,

    /// Set the baud rate.  The default is 9600 baud.
    pub baud: Option<usize>,
}

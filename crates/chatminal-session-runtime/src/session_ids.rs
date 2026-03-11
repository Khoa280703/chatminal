use std::fmt;

macro_rules! session_id_type {
    ($name:ident, $prefix:literal) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);

        impl $name {
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            pub const fn as_u64(self) -> u64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!($prefix, "-{}"), self.0)
            }
        }
    };
}

session_id_type!(SurfaceId, "surface");
session_id_type!(LeafId, "leaf");
session_id_type!(LayoutNodeId, "layout");

#[cfg(test)]
mod tests {
    use super::{LayoutNodeId, LeafId, SurfaceId};

    #[test]
    fn ids_format_with_stable_prefixes() {
        assert_eq!(SurfaceId::new(7).to_string(), "surface-7");
        assert_eq!(LeafId::new(11).to_string(), "leaf-11");
        assert_eq!(LayoutNodeId::new(13).to_string(), "layout-13");
    }
}

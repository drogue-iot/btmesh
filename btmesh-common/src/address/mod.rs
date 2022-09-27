pub mod group_address;
pub mod unicast_address;
pub mod virtual_address;

pub use group_address::GroupAddress;
pub use unicast_address::UnicastAddress;
pub use virtual_address::{LabelUuid, VirtualAddress};

use crate::ParseError;

/// Indicates an invalid address.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct InvalidAddress;

impl From<InvalidAddress> for ParseError {
    fn from(_: InvalidAddress) -> Self {
        ParseError::InvalidValue
    }
}

/// Represents any valid node address within a Bluetooth mesh network.
///
/// Additionally, represents a discrete LabelUuid, which is special, since
/// multiple LabelUuids can ambiguously surface as the same VirtualAddress.
///
/// The conversion from LabelUuid to VirtualAddress is deterministic, but the
/// inverse conversion from VirtualAddress to LabelUuid is not, without additional
/// network-specific information held by a given node.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Address {
    Unassigned,
    Unicast(UnicastAddress),
    Virtual(VirtualAddress),
    Group(GroupAddress),
}

impl Address {
    /// convert an Address to it's big-endian 2-byte array representation.
    pub fn as_bytes(&self) -> [u8; 2] {
        match self {
            Address::Unassigned => [0, 0],
            Address::Unicast(inner) => inner.as_bytes(),
            Address::Virtual(inner) => inner.as_bytes(),
            Address::Group(inner) => inner.as_bytes(),
        }
    }

    /// Parse a big-endian 2-byte array into a mesh address.
    pub fn parse(data: [u8; 2]) -> Self {
        let val = u16::from_be_bytes(data);
        if data[0] == 0 && data[1] == 0 {
            Self::Unassigned
        } else if UnicastAddress::is_unicast_address(&data) {
            // Safety: already performed the check.
            unsafe { Self::Unicast(UnicastAddress::new_unchecked(val)) }
        } else if GroupAddress::is_group_address(&data) {
            // Safety: already performed the check.
            unsafe { Self::Group(GroupAddress::new_unchecked(data)) }
        } else {
            // Safety: all previous checks cover all other cases.
            unsafe { Self::Virtual(VirtualAddress::new_unchecked(val)) }
        }
    }

    pub fn is_unicast(&self) -> bool {
        match self {
            Self::Unicast(_) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::address::{Address, GroupAddress, UnicastAddress, VirtualAddress};

    #[test]
    fn parse_unassigned() {
        assert_eq!(Address::parse([0x00, 0x00]), Address::Unassigned)
    }

    #[test]
    fn as_bytes_unassigned() {
        assert_eq!(Address::Unassigned.as_bytes(), [0x00, 0x00]);
    }

    #[test]
    fn parse_unicast() {
        unsafe {
            assert_eq!(
                Address::parse([0x00, 0x0A]),
                Address::Unicast(UnicastAddress::new_unchecked(0x00_0A))
            );
        }
    }

    #[test]
    fn as_bytes_unicast() {
        unsafe {
            assert_eq!(
                Address::Unicast(UnicastAddress::new_unchecked(0x00_0A)).as_bytes(),
                [0x00, 0x0A]
            );
        }
    }

    #[test]
    fn parse_virtual() {
        unsafe {
            assert_eq!(
                Address::parse([0x80, 0x0A]),
                Address::Virtual(VirtualAddress::new_unchecked(0x80_0A))
            );
        }
    }

    #[test]
    fn as_bytes_virtual() {
        unsafe {
            assert_eq!(
                Address::Virtual(VirtualAddress::new_unchecked(0x80_0A)).as_bytes(),
                [0x80, 0x0A]
            );
        }
    }

    #[test]
    fn parse_group() {
        assert_eq!(
            Address::parse([0xFF, 0xFC]),
            Address::Group(GroupAddress::AllProxies)
        );
        assert_eq!(
            Address::parse([0xFF, 0xFD]),
            Address::Group(GroupAddress::AllFriends)
        );
        assert_eq!(
            Address::parse([0xFF, 0xFE]),
            Address::Group(GroupAddress::AllRelays)
        );
        assert_eq!(
            Address::parse([0xFF, 0xFF]),
            Address::Group(GroupAddress::AllNodes)
        );
        assert_eq!(
            Address::parse([0xFF, 0x0A]),
            Address::Group(GroupAddress::RFU(0xFF_0A))
        );
        assert_eq!(
            Address::parse([0xC0, 0x00]),
            Address::Group(GroupAddress::Normal(0xC0_00))
        );
    }
}

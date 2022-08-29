use crate::address::{Address, InvalidAddress};

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GroupAddress {
    RFU(u16),
    Normal(u16),
    AllProxies,
    AllFriends,
    AllRelays,
    AllNodes,
}

impl GroupAddress {
    pub fn as_bytes(&self) -> [u8; 2] {
        match self {
            GroupAddress::RFU(bytes) => bytes.to_be_bytes(),
            GroupAddress::Normal(bytes) => bytes.to_be_bytes(),
            GroupAddress::AllProxies => [0xFF, 0xFC],
            GroupAddress::AllFriends => [0xFF, 0xFD],
            GroupAddress::AllRelays => [0xFF, 0xFE],
            GroupAddress::AllNodes => [0xFF, 0xFF],
        }
    }

    pub fn is_group_address(data: &[u8; 2]) -> bool {
        (data[0] & 0b11000000) == 0b11000000
    }

    pub fn parse(data: [u8; 2]) -> Result<Self, InvalidAddress> {
        if Self::is_group_address(&data) {
            // Safety: already checked
            unsafe { Ok(Self::new_unchecked(data)) }
        } else {
            Err(InvalidAddress)
        }
    }

    /// Parse an group address pattern.
    ///
    /// # Safety
    /// The bits must match the format of a group-address,
    /// otherwise, a non-group address bit pattern could be contained
    /// within.  See `is_group_address(...)`.
    pub unsafe fn new_unchecked(data: [u8; 2]) -> Self {
        match data {
            [0xFF, 0xFC] => Self::AllProxies,
            [0xFF, 0xFD] => Self::AllFriends,
            [0xFF, 0xFE] => Self::AllRelays,
            [0xFF, 0xFF] => Self::AllNodes,
            [0xFF, _] => Self::RFU(u16::from_be_bytes(data)),
            _ => Self::Normal(u16::from_be_bytes(data)),
        }
    }
}

impl From<GroupAddress> for Address {
    fn from(addr: GroupAddress) -> Self {
        Self::Group(addr)
    }
}

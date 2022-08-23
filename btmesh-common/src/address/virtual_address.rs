use crate::address::{Address, InvalidAddress};
use crate::{crypto, ParseError};
use cmac::crypto_mac::InvalidKeyLength;
use core::convert::TryInto;

/// A virtual address representing possibly several unique label UUIDs.
#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VirtualAddress(u16);

impl VirtualAddress {
    pub fn new(addr: u16) -> Result<Self, InvalidAddress> {
        if Self::is_virtual_address(&addr.to_be_bytes()) {
            Ok(Self(addr))
        } else {
            Err(InvalidAddress)
        }
    }

    /// Create a virtual address.
    ///
    /// # Safety
    /// The bit-pattern is not verified to be a valid virtual-address.
    pub unsafe fn new_unchecked(addr: u16) -> Self {
        Self(addr)
    }

    pub fn as_bytes(&self) -> [u8; 2] {
        self.0.to_be_bytes()
    }

    pub fn is_virtual_address(data: &[u8; 2]) -> bool {
        data[0] & 0b11000000 == 0b10000000
    }

    pub fn parse(data: [u8; 2]) -> Result<Self, InvalidAddress> {
        if Self::is_virtual_address(&data) {
            Ok(VirtualAddress(u16::from_be_bytes(data)))
        } else {
            Err(InvalidAddress)
        }
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for VirtualAddress {
    fn format(&self, fmt: defmt::Formatter) {
        let bytes = self.as_bytes();
        defmt::write!(fmt, "{:x}{:x}", bytes[0], bytes[1])
    }
}

impl From<VirtualAddress> for Address {
    fn from(addr: VirtualAddress) -> Self {
        Self::Virtual(addr)
    }
}

/// A unique label UUID used for virtual addresses to address multiple destinations.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LabelUuid {
    uuid: [u8; 16],
    address: VirtualAddress,
}

#[cfg(feature = "defmt")]
impl defmt::Format for LabelUuid {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "label={:x}{:x}{:x}{:x}{:x}{:x}{:x}{:x}{:x}{:x}{:x}{:x}{:x}{:x}{:x}{:x}; {}",
            self.uuid[0],
            self.uuid[1],
            self.uuid[2],
            self.uuid[3],
            self.uuid[4],
            self.uuid[5],
            self.uuid[6],
            self.uuid[7],
            self.uuid[8],
            self.uuid[9],
            self.uuid[10],
            self.uuid[11],
            self.uuid[12],
            self.uuid[13],
            self.uuid[14],
            self.uuid[15],
            self.address
        )
    }
}

impl LabelUuid {
    /// Parse a 16 octet label UUID.
    ///
    /// Returns a `ParseError` in the event the passed-in UUID is less than or longer than 16 octets.
    pub fn parse(uuid: &[u8]) -> Result<Self, ParseError> {
        if uuid.len() != 16 {
            Err(ParseError::InvalidLength)
        } else {
            Ok(
                Self::new(uuid.try_into().map_err(|_| ParseError::InvalidLength)?)
                    .map_err(|_| ParseError::InvalidLength)?,
            )
        }
    }

    pub fn new(uuid: [u8; 16]) -> Result<Self, InvalidKeyLength> {
        Ok(Self {
            uuid,
            address: Self::virtual_address_of(uuid)?,
        })
    }

    pub fn label_uuid(&self) -> &[u8] {
        &self.uuid
    }

    pub fn virtual_address(&self) -> VirtualAddress {
        self.address
    }

    pub fn virtual_address_of(uuid: [u8; 16]) -> Result<VirtualAddress, InvalidKeyLength> {
        let salt = crypto::s1(b"vtad")?;
        let hash = crypto::aes_cmac(&salt.into_bytes(), &uuid)?;
        let hash = &mut hash.into_bytes()[14..=15];
        hash[0] = (0b00111111 & hash[0]) | 0b10000000;
        let hash = u16::from_be_bytes([hash[0], hash[1]]);
        Ok(VirtualAddress(hash))
    }
}

#[cfg(test)]
mod tests {
    use crate::address::{LabelUuid, VirtualAddress};

    // Virtual addr: 800f, label: a04bf881e4a7bf702dfee1638ab8b2b3
    const UUID: [u8; 16] = [
        0xa0, 0x4b, 0xf8, 0x81, 0xe4, 0xa7, 0xbf, 0x70, 0x2d, 0xfe, 0xe1, 0x63, 0x8a, 0xb8, 0xb2,
        0xb3,
    ];
    const ADDR: u16 = 0x800f;

    #[test]
    fn virtual_address() {
        let label = LabelUuid::new(UUID);
        unsafe {
            assert_eq!(
                label.unwrap().virtual_address(),
                VirtualAddress::new_unchecked(ADDR)
            )
        }
    }

    #[test]
    fn virtual_address_of() {
        unsafe {
            assert_eq!(
                LabelUuid::virtual_address_of(UUID),
                Ok(VirtualAddress::new_unchecked(ADDR))
            )
        }
    }
}

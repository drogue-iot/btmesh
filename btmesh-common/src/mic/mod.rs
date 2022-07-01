use crate::ParseError;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SzMic {
    Bit32,
    Bit64,
}

impl SzMic {
    pub fn size(&self) -> usize {
        match self {
            Self::Bit32 => 4,
            Self::Bit64 => 8,
        }
    }

    pub fn parse(data: u8) -> Self {
        if data != 0 {
            Self::Bit64
        } else {
            Self::Bit32
        }
    }
}

/// Error signifying an invalid length of MIC provided to TransMic.
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct InvalidLength;

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TransMic {
    Bit32(Bit32TransMic),
    Bit64(Bit64TransMic),
}

impl AsRef<[u8]> for TransMic {
    fn as_ref(&self) -> &[u8] {
        match self {
            TransMic::Bit32(inner) => inner.as_ref(),
            TransMic::Bit64(inner) => inner.as_ref(),
        }
    }
}

impl AsMut<[u8]> for TransMic {
    fn as_mut(&mut self) -> &mut [u8] {
        match self {
            TransMic::Bit32(inner) => inner.as_mut(),
            TransMic::Bit64(inner) => inner.as_mut(),
        }
    }
}

impl TransMic {
    pub fn new32() -> Self {
        Self::Bit32(Bit32TransMic::new())
    }

    pub fn new64() -> Self {
        Self::Bit64(Bit64TransMic::new())
    }

    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        match data.len() {
            4 => Ok(TransMic::Bit32(Bit32TransMic(data.try_into()?))),
            8 => Ok(TransMic::Bit64(Bit64TransMic(data.try_into()?))),
            _ => Err(ParseError::InvalidLength),
        }
    }

    pub fn szmic(&self) -> SzMic {
        match self {
            TransMic::Bit32(_) => SzMic::Bit32,
            TransMic::Bit64(_) => SzMic::Bit64,
        }
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Bit32TransMic([u8; 4]);

impl Bit32TransMic {
    pub fn new() -> Self {
        Self([0; 4])
    }
}

impl Default for Bit32TransMic {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<[u8]> for Bit32TransMic {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for Bit32TransMic {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Bit64TransMic([u8; 8]);

impl Bit64TransMic {
    pub fn new() -> Self {
        Self([0; 8])
    }
}

impl Default for Bit64TransMic {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<[u8]> for Bit64TransMic {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for Bit64TransMic {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::mic::{SzMic, TransMic};

    #[test]
    fn transmic_parse() {
        let transmic = TransMic::parse(b"abcd").unwrap();
        if let TransMic::Bit32(transmic) = transmic {
            assert_eq!(*b"abcd", transmic.as_ref())
        } else {
            assert!(false, "failed to parse a 32-bit transmic")
        }

        let transmic = TransMic::parse(b"abcdefgh").unwrap();

        if let TransMic::Bit64(transmic) = transmic {
            assert_eq!(*b"abcdefgh", transmic.as_ref())
        } else {
            assert!(false, "failed to parse a 64-bit transmic")
        }

        if let Err(_) = TransMic::parse(b"") {
            // okay
        } else {
            assert!(false, "failed to error on 0-byte transmic")
        }

        if let Err(_) = TransMic::parse(b"a") {
            // okay
        } else {
            assert!(false, "failed to error on 1-byte transmic")
        }

        if let Err(_) = TransMic::parse(b"ab") {
            // okay
        } else {
            assert!(false, "failed to error on 2-byte transmic")
        }

        if let Err(_) = TransMic::parse(b"abc") {
            // okay
        } else {
            assert!(false, "failed to error on 3-byte transmic")
        }

        if let Err(_) = TransMic::parse(b"abcde") {
            // okay
        } else {
            assert!(false, "failed to error on 5-byte transmic")
        }
    }

    #[test]
    fn transmic_szmic() {
        let transmic = TransMic::parse(b"abcd").unwrap();

        assert!(matches!(transmic, TransMic::Bit32(_)));
        assert_eq!(SzMic::Bit32, transmic.szmic());

        let transmic = TransMic::parse(b"abcdwxyz").unwrap();

        assert!(matches!(transmic, TransMic::Bit64(_)));
        assert_eq!(SzMic::Bit64, transmic.szmic());
    }

    #[test]
    fn szmic_len() {
        assert_eq!(4, SzMic::Bit32.size());
        assert_eq!(8, SzMic::Bit64.size());
    }

    #[test]
    fn szmic_parse() {
        assert_eq!(SzMic::Bit32, SzMic::parse(0));
        assert_eq!(SzMic::Bit64, SzMic::parse(1));
    }
}

use crate::InsufficientBuffer;
use heapless::Vec;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Opcode {
    OneOctet(u8),
    TwoOctet(u8, u8),
    ThreeOctet(u8, u8, u8),
}

impl Opcode {
    pub fn matches(&self, data: &[u8]) -> bool {
        match self {
            Opcode::OneOctet(a) if !data.is_empty() && data[0] == *a => true,
            Opcode::TwoOctet(a, b) if data.len() >= 2 && data[0] == *a && data[1] == *b => true,
            Opcode::ThreeOctet(a, b, c)
                if data.len() >= 3 && data[0] == *a && data[1] == *b && data[2] == *c =>
            {
                true
            }
            _ => false,
        }
    }

    pub fn opcode_len(&self) -> usize {
        match self {
            Opcode::OneOctet(_) => 1,
            Opcode::TwoOctet(_, _) => 2,
            Opcode::ThreeOctet(_, _, _) => 3,
        }
    }

    pub fn split(data: &[u8]) -> Option<(Opcode, &[u8])> {
        if data.is_empty() {
            None
        } else if data[0] & 0b10000000 == 0 {
            // one octet
            Some((Opcode::OneOctet(data[0] & 0b01111111), &data[1..]))
        } else if data.len() >= 2 && data[0] & 0b11000000 == 0b10000000 {
            // two octet
            Some((Opcode::TwoOctet(data[0], data[1]), &data[2..]))
        } else if data.len() >= 3 && data[0] & 0b11000000 == 0b11000000 {
            // three octet
            Some((Opcode::ThreeOctet(data[0], data[1], data[2]), &data[3..]))
        } else {
            None
        }
    }

    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        match self {
            Opcode::OneOctet(a) => {
                xmit.push(*a).map_err(|_| InsufficientBuffer)?;
            }
            Opcode::TwoOctet(a, b) => {
                xmit.push(*a).map_err(|_| InsufficientBuffer)?;
                xmit.push(*b).map_err(|_| InsufficientBuffer)?;
            }
            Opcode::ThreeOctet(a, b, c) => {
                xmit.push(*a).map_err(|_| InsufficientBuffer)?;
                xmit.push(*b).map_err(|_| InsufficientBuffer)?;
                xmit.push(*c).map_err(|_| InsufficientBuffer)?;
            }
        }
        Ok(())
    }
}

#[allow(unused)]
#[cfg(feature = "defmt")]
impl defmt::Format for Opcode {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            Opcode::OneOctet(a) => {
                defmt::write!(fmt, "{:02x}", a)
            }
            Opcode::TwoOctet(a, b) => {
                defmt::write!(fmt, "{:02x}{:02x}", a, b)
            }
            Opcode::ThreeOctet(a, b, c) => {
                defmt::write!(fmt, "{:02x}{:02x}{:02x}", a, b, c)
            }
        }
    }
}

#[macro_export]
macro_rules! opcode {
    ($name:ident $o1:expr) => {
        pub const $name: $crate::opcode::Opcode = $crate::opcode::Opcode::OneOctet($o1);
    };

    ($name:ident $o1:expr, $o2:expr) => {
        pub const $name: $crate::opcode::Opcode = $crate::opcode::Opcode::TwoOctet($o1, $o2);
    };

    ($name:ident $o1:expr, $o2:expr, $o3:expr) => {
        pub const $name: $crate::opcode::Opcode = $crate::opcode::Opcode::ThreeOctet($o1, $o2, $o3);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_codec() {
        let opcodes = vec![
            Opcode::OneOctet(0x52),
            Opcode::TwoOctet(0x82, 0x31),
            Opcode::ThreeOctet(0xC2, 0x31, 0x11),
        ];

        for opcode in opcodes.iter() {
            let mut v: heapless::Vec<u8, 4> = Vec::new();
            opcode.emit(&mut v).unwrap();

            let (decoded, _) = Opcode::split(&v[..]).unwrap();
            assert_eq!(decoded, *opcode);
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Location(u16);

impl Location {
    pub fn numeric(number: u8) -> Self {
        Self(number as u16)
    }

    pub fn to_le_bytes(&self) -> [u8; 2] {
        self.0.to_le_bytes()
    }
}

#[macro_export]
macro_rules! location {
    ($name: ident, $val: expr) => {
        pub const $name: Location = Location($val);
    };
}

// https://btprodspecificationrefs.blob.core.windows.net/assigned-numbers/Assigned%20Number%20Types/GATT%20Namespace%20Descriptors.pdf

location!(AUXILARY, 0x0108);
location!(BACK, 0x0101);
location!(BACKUP, 0x0107);
location!(BOTTOM, 0x0103);
location!(EXTERNAL, 0x0110);
location!(FLASH, 0x010A);
location!(FRONT, 0x0100);
location!(INSIDE, 0x010B);
location!(INTERNAL, 0x010F);
location!(LEFT, 0x010D);
location!(LOWER, 0x0105);
location!(MAIN, 0x0106);
location!(OUTSIDE, 0x010C);
location!(RIGHT, 0x010E);
location!(SUPPLEMENTARY, 0x0109);
location!(TOP, 0x0102);
location!(UNKNOWN, 0x0000);
location!(UPPER, 0x0104);

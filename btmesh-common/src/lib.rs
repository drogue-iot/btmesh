#![cfg_attr(not(test), no_std)]

use core::array::TryFromSliceError;
use core::ops::{Add, BitAnd, Deref, Index, IndexMut, Sub};
use heapless::Vec;
use rand_core::RngCore;

use crate::location::Location;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub mod address;
pub mod crc;
pub mod crypto;
pub mod location;
pub mod mic;
pub mod opcode;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct InsufficientBuffer;

impl From<()> for InsufficientBuffer {
    fn from(_: ()) -> Self {
        InsufficientBuffer
    }
}

impl From<u8> for InsufficientBuffer {
    fn from(_: u8) -> Self {
        InsufficientBuffer
    }
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    InvalidPDUFormat,
    InvalidValue,
    InvalidLength,
    InsufficientBuffer,
}

impl From<()> for ParseError {
    fn from(_: ()) -> Self {
        Self::InsufficientBuffer
    }
}

impl From<TryFromSliceError> for ParseError {
    fn from(_: TryFromSliceError) -> Self {
        Self::InvalidLength
    }
}

impl From<cmac::crypto_mac::InvalidKeyLength> for ParseError {
    fn from(_: cmac::crypto_mac::InvalidKeyLength) -> Self {
        Self::InvalidLength
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Default, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum IvUpdateFlag {
    #[default]
    Normal,
    InProgress,
}

impl IvUpdateFlag {
    pub fn parse(data: u8) -> Self {
        if data == 0 {
            Self::Normal
        } else {
            Self::InProgress
        }
    }
    pub fn emit(&self, data: &mut u8) {
        if self == &Self::InProgress {
            *data |= 0b00000010;
        }
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct KeyRefreshFlag(pub bool);

impl KeyRefreshFlag {
    pub fn parse(data: u8) -> Self {
        Self(data != 0)
    }
    pub fn emit(&self, data: &mut u8) {
        if self.0 {
            *data |= 0b00000001;
        }
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IvIndex(u32);

#[cfg(feature = "defmt")]
impl ::defmt::Format for IvIndex {
    fn format(&self, fmt: ::defmt::Formatter) {
        ::defmt::write!(fmt, "{}", self.0)
    }
}

impl IvIndex {
    pub fn new(iv_index: u32) -> Self {
        Self(iv_index)
    }

    pub fn parse(iv_index: &[u8]) -> Result<Self, ParseError> {
        if iv_index.len() == 4 {
            Ok(Self(u32::from_be_bytes([
                iv_index[0],
                iv_index[1],
                iv_index[2],
                iv_index[3],
            ])))
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    pub fn to_be_bytes(&self) -> [u8; 4] {
        self.0.to_be_bytes()
    }

    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn ivi(&self) -> Ivi {
        if self.0 & 1 == 1 {
            Ivi::One
        } else {
            Ivi::Zero
        }
    }

    pub fn accepted_iv_index(&self, ivi: Ivi) -> IvIndex {
        if self.ivi() == ivi {
            *self
        } else {
            *self - 1
        }
    }

    pub fn transmission_iv_index(&self, flag: IvUpdateFlag) -> IvIndex {
        match flag {
            IvUpdateFlag::Normal => *self,
            IvUpdateFlag::InProgress => *self - 1,
        }
    }
}

impl Sub<u8> for IvIndex {
    type Output = Self;

    fn sub(self, rhs: u8) -> Self::Output {
        if self.0 > rhs as u32 {
            Self(self.0 - rhs as u32)
        } else {
            self
        }
    }
}

impl Add<u8> for IvIndex {
    type Output = Self;

    fn add(self, rhs: u8) -> Self::Output {
        Self(self.0 + rhs as u32)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Ivi {
    Zero,
    One,
}

impl Ivi {
    pub fn parse(ivi: u8) -> Result<Ivi, ParseError> {
        match ivi {
            0 => Ok(Ivi::Zero),
            1 => Ok(Ivi::One),
            _ => Err(ParseError::InvalidValue),
        }
    }
}

impl From<Ivi> for u8 {
    fn from(ivi: Ivi) -> Self {
        match ivi {
            Ivi::Zero => 0,
            Ivi::One => 1,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Ttl(u8);

impl Ttl {
    pub fn new(ttl: u8) -> Self {
        Self(ttl)
    }

    pub fn parse(ttl: u8) -> Result<Self, ParseError> {
        Ok(Self(ttl))
    }

    pub fn value(&self) -> u8 {
        self.0
    }

    pub fn decr(&self) -> Self {
        if self.0 > 1 {
            Self(self.0 - 1)
        } else {
            Self(0)
        }
    }
}

pub struct SeqRolloverError;

#[derive(Default, Copy, Clone, Eq, PartialEq, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Seq(u32);

impl Add<u32> for Seq {
    type Output = Result<Seq, SeqRolloverError>;

    fn add(self, rhs: u32) -> Self::Output {
        match self.0.checked_add(rhs) {
            None => Err(SeqRolloverError),
            Some(val) => Ok(Self(val)),
        }
    }
}

impl Seq {
    pub fn new(seq: u32) -> Self {
        Self(seq)
    }

    pub fn parse(seq: u32) -> Result<Seq, ParseError> {
        Ok(Self(seq))
    }

    pub fn to_be_bytes(&self) -> [u8; 4] {
        self.0.to_be_bytes()
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl From<Seq> for SeqZero {
    fn from(seq: Seq) -> Self {
        Self(seq.0 as u16)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SeqZero(u16);

impl SeqZero {
    pub fn new(seq_zero: u16) -> Self {
        Self(seq_zero)
    }

    pub fn parse(data: u16) -> Result<Self, ParseError> {
        Ok(Self(data))
    }

    pub fn value(&self) -> u16 {
        self.0
    }
}

impl BitAnd<u16> for SeqZero {
    type Output = u16;

    fn bitand(self, rhs: u16) -> Self::Output {
        self.0 & rhs
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SeqAuth(u32);

impl SeqAuth {
    pub fn new(seq_auth: u32) -> Self {
        Self(seq_auth)
    }
}

impl From<SeqAuth> for Seq {
    fn from(seq_auth: SeqAuth) -> Self {
        Seq::new(seq_auth.0)
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Ctl {
    Access,
    Control,
}

impl Ctl {
    pub fn parse(ctl: u8) -> Result<Ctl, ParseError> {
        match ctl {
            0 => Ok(Ctl::Access),
            1 => Ok(Ctl::Control),
            _ => Err(ParseError::InvalidValue),
        }
    }

    pub fn netmic_size(&self) -> usize {
        match self {
            Ctl::Access => 4,
            Ctl::Control => 8,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Uuid([u8; 16]);

#[cfg(feature = "defmt")]
impl defmt::Format for Uuid {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "{=u8:02X}{=u8:02X}{=u8:02X}{=u8:02X}{=u8:02X}{=u8:02X}{=u8:02X}{=u8:02X}{=u8:02X}{=u8:02X}{=u8:02X}{=u8:02X}{=u8:02X}{=u8:02X}{=u8:02X}{=u8:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5], self.0[6], self.0[7], self.0[8], self.0[9], self.0[10], self.0[11], self.0[12], self.0[13], self.0[14], self.0[15],
        )
    }
}

impl Uuid {
    pub fn new(uuid: [u8; 16]) -> Self {
        Self(uuid)
    }

    pub fn new_random<R: RngCore>(rng: &mut R) -> Self {
        let mut bytes = [0; 16];
        rng.fill_bytes(&mut bytes);
        Self(
            *uuid::Builder::from_random_bytes(bytes)
                .into_uuid()
                .as_bytes(),
        )
    }
}

impl Deref for Uuid {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NetworkId([u8; 8]);

impl NetworkId {
    pub fn new(network_id: [u8; 8]) -> Self {
        Self(network_id)
    }
}

impl Deref for NetworkId {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "darling", derive(darling::FromMeta))]
pub struct CompanyIdentifier(pub u16);

impl CompanyIdentifier {
    pub fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 2 {
            Ok(Self(u16::from_le_bytes([parameters[0], parameters[1]])))
        } else {
            Err(ParseError::InvalidLength)
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "darling", derive(darling::FromMeta))]
pub struct ProductIdentifier(pub u16);

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "darling", derive(darling::FromMeta))]
pub struct VersionIdentifier(pub u16);

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum ModelIdentifier {
    SIG(u16),
    Vendor(CompanyIdentifier, u16),
}

impl ModelIdentifier {
    pub fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() == 2 {
            Ok(ModelIdentifier::SIG(u16::from_le_bytes([
                parameters[0],
                parameters[1],
            ])))
        } else if parameters.len() == 4 {
            Ok(ModelIdentifier::Vendor(
                CompanyIdentifier::parse(&parameters[0..=1])?,
                u16::from_le_bytes([parameters[2], parameters[3]]),
            ))
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        // NOTE: While so many things are big-endian... this is little-endian.
        // WHY OH WHY?
        match self {
            ModelIdentifier::SIG(model_id) => {
                xmit.extend_from_slice(&model_id.to_le_bytes())
                    .map_err(|_| InsufficientBuffer)?;
            }
            ModelIdentifier::Vendor(company_id, model_id) => {
                xmit.extend_from_slice(&company_id.0.to_le_bytes())
                    .map_err(|_| InsufficientBuffer)?;
                xmit.extend_from_slice(&model_id.to_le_bytes())
                    .map_err(|_| InsufficientBuffer)?;
            }
        }
        Ok(())
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for ModelIdentifier {
    fn format(&self, fmt: defmt::Formatter) {
        match *self {
            ModelIdentifier::SIG(id) => {
                defmt::write!(fmt, "SIG(0x{=u16:04x})", id);
            }
            ModelIdentifier::Vendor(company_id, model_id) => {
                defmt::write!(fmt, "Vendor({}, 0x{=u16:04x})", company_id, model_id);
            }
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Composition<X: Default = ()> {
    pub(crate) cid: CompanyIdentifier,
    pub(crate) pid: ProductIdentifier,
    pub(crate) vid: VersionIdentifier,
    pub(crate) crpl: u16, // Count Replay Protection List
    pub(crate) features: Features,
    pub(crate) elements: Vec<ElementDescriptor<X>, 4>,
}

impl<X: Default> Index<u8> for Composition<X> {
    type Output = ElementDescriptor<X>;

    fn index(&self, index: u8) -> &Self::Output {
        &self.elements[index as usize]
    }
}

impl<X: Default> IndexMut<u8> for Composition<X> {
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        &mut self.elements[index as usize]
    }
}

impl<X: Default> Composition<X> {
    pub fn new(cid: CompanyIdentifier, pid: ProductIdentifier, vid: VersionIdentifier) -> Self {
        Self {
            cid,
            pid,
            vid,
            crpl: 0,
            features: Features::default(),
            elements: Default::default(),
        }
    }

    pub fn simplify(&self) -> Composition<()> {
        Composition {
            cid: self.cid,
            pid: self.pid,
            vid: self.vid,
            crpl: self.crpl,
            features: self.features,
            elements: self.elements.iter().map(|e| e.simplify()).collect(),
        }
    }

    pub fn add_element(
        &mut self,
        element: ElementDescriptor<X>,
    ) -> Result<(), ElementDescriptor<X>> {
        self.elements.push(element)
    }

    pub fn number_of_elements(&self) -> u8 {
        self.elements.len() as u8
    }

    pub fn cid(&self) -> CompanyIdentifier {
        self.cid
    }

    pub fn pid(&self) -> ProductIdentifier {
        self.pid
    }

    pub fn vid(&self) -> VersionIdentifier {
        self.vid
    }

    pub fn crpl(&self) -> u16 {
        self.crpl
    }

    pub fn features(&self) -> Features {
        self.features
    }

    pub fn elements_iter(&self) -> impl Iterator<Item = &ElementDescriptor<X>> + '_ {
        self.elements.iter()
    }

    pub fn elements_iter_mut(&mut self) -> impl Iterator<Item = &mut ElementDescriptor<X>> + '_ {
        self.elements.iter_mut()
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ElementDescriptor<X: Default = ()> {
    pub loc: Location,
    pub models: Vec<ModelDescriptor<X>, 4>,
}

impl<X: Default> ElementDescriptor<X> {
    pub fn new(loc: Location) -> Self {
        Self {
            loc,
            models: Default::default(),
        }
    }

    pub fn simplify(&self) -> ElementDescriptor<()> {
        ElementDescriptor {
            loc: self.loc,
            models: self
                .models
                .iter()
                .map(|e| ModelDescriptor {
                    model_identifier: e.model_identifier,
                    extra: (),
                })
                .collect(),
        }
    }

    pub fn add_model(&mut self, model_identifier: ModelIdentifier) {
        self.models
            .push(ModelDescriptor {
                model_identifier,
                extra: X::default(),
            })
            .ok();
    }

    pub fn loc(&self) -> Location {
        self.loc
    }

    pub fn models_iter(&self) -> impl Iterator<Item = &ModelDescriptor<X>> + '_ {
        self.models.iter()
    }

    pub fn models_iter_mut(&mut self) -> impl Iterator<Item = &mut ModelDescriptor<X>> + '_ {
        self.models.iter_mut()
    }

    pub fn has_model(&self, model_identifier: ModelIdentifier) -> bool {
        self.models
            .iter()
            .any(|e| e.model_identifier == model_identifier)
    }
}

impl<X: Default> Index<u8> for ElementDescriptor<X> {
    type Output = ModelDescriptor<X>;

    fn index(&self, index: u8) -> &Self::Output {
        &self.models[index as usize]
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ModelDescriptor<X = ()> {
    pub model_identifier: ModelIdentifier,
    pub extra: X,
}

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Features {
    pub relay: bool,
    pub proxy: bool,
    pub friend: bool,
    pub low_power: bool,
}

impl Features {
    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        // bits 15-8 RFU
        let mut val = 0;
        if self.relay {
            val |= 0b0001;
        }
        if self.proxy {
            val |= 0b0010;
        }
        if self.friend {
            val |= 0b0100;
        }
        if self.low_power {
            val |= 0b1000;
        }
        xmit.push(val).map_err(|_| InsufficientBuffer)?;
        xmit.push(0).map_err(|_| InsufficientBuffer)?;
        Ok(())
    }
}

#[allow(clippy::derivable_impls)]
impl Default for Features {
    fn default() -> Self {
        Self {
            #[cfg(feature = "relay")]
            relay: true,
            #[cfg(not(feature = "relay"))]
            relay: false,
            #[cfg(feature = "proxy")]
            proxy: true,
            #[cfg(not(feature = "proxy"))]
            proxy: false,
            #[cfg(feature = "friend")]
            friend: true,
            #[cfg(not(feature = "friend"))]
            friend: false,
            #[cfg(feature = "low_power")]
            low_power: true,
            #[cfg(not(feature = "low_power"))]
            low_power: false,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{IvIndex, IvUpdateFlag, Ivi};

    #[test]
    fn iv_index_zero() {
        let iv_index = IvIndex::parse(&[0x00, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(Ivi::Zero, iv_index.ivi());

        assert_eq!(iv_index, iv_index.accepted_iv_index(Ivi::Zero));
        // special case, non-valid but don't break.
        assert_eq!(iv_index, iv_index.accepted_iv_index(Ivi::One));

        assert_eq!(
            iv_index,
            iv_index.transmission_iv_index(IvUpdateFlag::Normal)
        );

        // special case, non-valid but don't break.
        assert_eq!(
            iv_index,
            iv_index.transmission_iv_index(IvUpdateFlag::InProgress)
        );
    }

    #[test]
    fn iv_index_non_zero() {
        let iv_index = IvIndex::parse(&[0x00, 0x00, 0x00, 0x03]).unwrap();
        let prev_iv_index = iv_index - 1;

        assert_eq!(iv_index.value(), 3);
        assert_eq!(prev_iv_index.value(), 2);

        assert_eq!(Ivi::One, iv_index.ivi());

        assert_eq!(iv_index, iv_index.accepted_iv_index(Ivi::One));
        assert_eq!(prev_iv_index, iv_index.accepted_iv_index(Ivi::Zero));
    }
}

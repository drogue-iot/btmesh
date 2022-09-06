use crate::{Message, Model};
use btmesh_common::opcode::Opcode;
use btmesh_common::{opcode, InsufficientBuffer, ModelIdentifier, ParseError};
use heapless::Vec;
#[allow(unused_imports)]
use micromath::F32Ext;

#[derive(Clone, Debug, Default)]
pub struct SensorClient<C, const NUM_SENSORS: usize, const NUM_COLUMNS: usize>
where
    C: SensorConfig,
{
    _c: core::marker::PhantomData<C>,
}

#[derive(Clone, Debug, Default)]
pub struct SensorServer<C, const NUM_SENSORS: usize, const NUM_COLUMNS: usize>
where
    C: SensorConfig,
{
    _c: core::marker::PhantomData<C>,
}

impl<C, const NUM_SENSORS: usize, const NUM_COLUMNS: usize>
    SensorServer<C, NUM_SENSORS, NUM_COLUMNS>
where
    C: SensorConfig,
{
    pub fn new() -> Self {
        Self {
            _c: core::marker::PhantomData,
        }
    }
}

impl<C, const NUM_SENSORS: usize, const NUM_COLUMNS: usize>
    SensorClient<C, NUM_SENSORS, NUM_COLUMNS>
where
    C: SensorConfig,
{
    pub fn new() -> Self {
        Self {
            _c: core::marker::PhantomData,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SensorSetupServer<C, const NUM_SENSORS: usize, const NUM_COLUMNS: usize>
where
    C: SensorSetupConfig,
{
    _server: SensorServer<C, NUM_SENSORS, NUM_COLUMNS>,
}

impl<C, const NUM_SENSORS: usize, const NUM_COLUMNS: usize>
    SensorSetupServer<C, NUM_SENSORS, NUM_COLUMNS>
where
    C: SensorSetupConfig,
{
    pub fn new() -> Self {
        Self {
            _server: SensorServer::new(),
        }
    }
}

impl<C, const NUM_SENSORS: usize, const NUM_COLUMNS: usize> Default
    for SensorSetupServer<C, NUM_SENSORS, NUM_COLUMNS>
where
    C: SensorSetupConfig,
{
    fn default() -> Self {
        Self::new()
    }
}

pub const SENSOR_SERVER: ModelIdentifier = ModelIdentifier::SIG(0x1100);
pub const SENSOR_SETUP_SERVER: ModelIdentifier = ModelIdentifier::SIG(0x1101);
pub const SENSOR_CLIENT: ModelIdentifier = ModelIdentifier::SIG(0x1102);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PropertyId(pub u16);

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RawValue(pub Vec<u8, 128>);

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Tolerance(pub u16);

#[cfg(feature = "defmt")]
pub trait SensorConfig: defmt::Format + Clone {
    type Data: SensorData + defmt::Format + core::fmt::Debug;
    const DESCRIPTORS: &'static [SensorDescriptor];
}

#[cfg(not(feature = "defmt"))]
pub trait SensorConfig: Clone {
    type Data: SensorData + core::fmt::Debug;
    const DESCRIPTORS: &'static [SensorDescriptor];
}

pub trait SensorData: Default {
    fn decode(&mut self, property: PropertyId, data: &[u8]) -> Result<(), ParseError>;
    fn encode<const N: usize>(
        &self,
        property: PropertyId,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer>;
}

pub trait SensorSetupConfig: SensorConfig {
    const CADENCE_DESCRIPTORS: &'static [CadenceDescriptor];
    const SETTING_DESCRIPTORS: &'static [SettingDescriptor];
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DescriptorGet {
    id: Option<PropertyId>,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DescriptorStatus<const NUM_SENSORS: usize> {
    NotFound(PropertyId),
    Descriptors(Vec<SensorDescriptor, NUM_SENSORS>),
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SensorDescriptor {
    pub id: PropertyId,
    pub positive_tolerance: Tolerance,
    pub negative_tolerance: Tolerance,
    pub sampling_function: SamplingFunction,
    pub measurement_period: Option<u32>,
    pub update_interval: Option<u32>,
    pub size: usize,
    pub x_size: usize,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CadenceDescriptor {
    id: PropertyId,
    size: usize,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SettingDescriptor {
    sensor: PropertyId,
    setting: PropertyId,
    size: usize,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SamplingFunction {
    Unspecified,
    Instantaneous,
    ArithmeticMean,
    RMS,
    Maximum,
    Minimum,
    Accumulated,
    Count,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SensorGet {
    id: Option<PropertyId>,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SensorStatus<C>
where
    C: SensorConfig,
{
    pub data: C::Data,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ColumnGet {
    id: PropertyId,
    x: RawValue,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ColumnStatus {
    id: PropertyId,
    x: RawValue,
    values: Option<(RawValue, RawValue)>,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SeriesGet {
    id: PropertyId,
    x: Option<(RawValue, RawValue)>,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SeriesStatus<const NUM_COLUMNS: usize> {
    id: PropertyId,
    values: Vec<(RawValue, RawValue, RawValue), NUM_COLUMNS>,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SensorMessage<C, const NUM_SENSORS: usize, const NUM_COLUMNS: usize>
where
    C: SensorConfig,
{
    DescriptorGet(DescriptorGet),
    DescriptorStatus(DescriptorStatus<NUM_SENSORS>),
    Get(SensorGet),
    Status(SensorStatus<C>),
    ColumnGet(ColumnGet),
    ColumnStatus(ColumnStatus),
    SeriesGet(SeriesGet),
    SeriesStatus(SeriesStatus<NUM_COLUMNS>),
    CadenceGet(CadenceGet),
    CadenceSet(CadenceSet),
    CadenceSetUnacknowledged(CadenceSet),
    CadenceStatus(CadenceStatus),
    SettingsGet(SettingsGet),
    SettingsStatus(SettingsStatus<NUM_SENSORS>),
    SettingGet(SettingGet),
    SettingSet(SettingSet),
    SettingSetUnacknowledged(SettingSet),
    SettingStatus(SettingStatus),
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CadenceGet {
    id: PropertyId,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CadenceSet {
    id: PropertyId,
    fast_cadence_divisor: u8,
    status_trigger_type: StatusTriggerType,
    status_trigger_delta_down: RawValue,
    status_trigger_delta_up: RawValue,
    status_min_interval: u8,
    fast_cadence_low: RawValue,
    fast_cadence_high: RawValue,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum StatusTriggerType {
    Property,
    Unitless,
}

pub type CadenceStatus = CadenceSet;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SettingsGet {
    id: PropertyId,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SettingsStatus<const NUM_SENSORS: usize> {
    id: PropertyId,
    settings: Vec<PropertyId, NUM_SENSORS>,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SettingGet {
    id: PropertyId,
    setting: PropertyId,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SettingSet {
    id: PropertyId,
    setting: PropertyId,
    raw: RawValue,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SettingStatus {
    id: PropertyId,
    setting: PropertyId,
    access: SensorSettingAccess,
    raw: RawValue,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SensorSettingAccess {
    Read,
    ReadWrite,
}

opcode!( SENSOR_DESCRIPTOR_GET 0x82, 0x30 );
opcode!( SENSOR_DESCRIPTOR_STATUS 0x51 );
opcode!( SENSOR_GET 0x82, 0x31 );
opcode!( SENSOR_STATUS 0x52 );
opcode!( SENSOR_COLUMN_GET 0x82, 0x32 );
opcode!( SENSOR_COLUMN_STATUS 0x53 );
opcode!( SENSOR_SERIES_GET 0x82, 0x33 );
opcode!( SENSOR_SERIES_STATUS 0x54 );

opcode!( SENSOR_CADENCE_GET 0x82, 0x34 );
opcode!( SENSOR_CADENCE_SET 0x55 );
opcode!( SENSOR_CADENCE_SET_UNACKNOWLEDGED 0x56 );
opcode!( SENSOR_CADENCE_STATUS 0x57 );
opcode!( SENSOR_SETTINGS_GET 0x82, 0x35 );
opcode!( SENSOR_SETTINGS_STATUS 0x58 );
opcode!( SENSOR_SETTING_GET 0x82, 0x36 );
opcode!( SENSOR_SETTING_SET 0x59 );
opcode!( SENSOR_SETTING_SET_UNACKNOWLEDGED 0x5A );
opcode!( SENSOR_SETTING_STATUS 0x5B );

impl<C, const NUM_SENSORS: usize, const NUM_COLUMNS: usize> Message
    for SensorMessage<C, NUM_SENSORS, NUM_COLUMNS>
where
    C: SensorConfig,
{
    fn opcode(&self) -> Opcode {
        match self {
            Self::DescriptorGet(_) => SENSOR_DESCRIPTOR_GET,
            Self::DescriptorStatus(_) => SENSOR_DESCRIPTOR_STATUS,
            Self::Get(_) => SENSOR_GET,
            Self::Status(_) => SENSOR_STATUS,
            Self::ColumnGet(_) => SENSOR_COLUMN_GET,
            Self::ColumnStatus(_) => SENSOR_COLUMN_STATUS,
            Self::SeriesGet(_) => SENSOR_SERIES_GET,
            Self::SeriesStatus(_) => SENSOR_SERIES_STATUS,
            Self::CadenceGet(_) => SENSOR_CADENCE_GET,
            Self::CadenceSet(_) => SENSOR_CADENCE_SET,
            Self::CadenceSetUnacknowledged(_) => SENSOR_CADENCE_SET_UNACKNOWLEDGED,
            Self::CadenceStatus(_) => SENSOR_CADENCE_STATUS,
            Self::SettingsGet(_) => SENSOR_SETTINGS_GET,
            Self::SettingsStatus(_) => SENSOR_SETTINGS_STATUS,
            Self::SettingGet(_) => SENSOR_SETTING_GET,
            Self::SettingSet(_) => SENSOR_SETTING_SET,
            Self::SettingSetUnacknowledged(_) => SENSOR_SETTING_SET_UNACKNOWLEDGED,
            Self::SettingStatus(_) => SENSOR_SETTING_STATUS,
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        match self {
            Self::DescriptorGet(m) => m.emit_parameters(xmit),
            Self::DescriptorStatus(m) => m.emit_parameters(xmit),
            Self::Get(m) => m.emit_parameters(xmit),
            Self::Status(m) => m.emit_parameters(xmit),
            Self::ColumnGet(m) => m.emit_parameters(xmit),
            Self::ColumnStatus(m) => m.emit_parameters(xmit),
            Self::SeriesGet(m) => m.emit_parameters(xmit),
            Self::SeriesStatus(m) => m.emit_parameters(xmit),
            Self::CadenceGet(m) => m.emit_parameters(xmit),
            Self::CadenceSet(m) => m.emit_parameters(xmit),
            Self::CadenceSetUnacknowledged(m) => m.emit_parameters(xmit),
            Self::CadenceStatus(m) => m.emit_parameters(xmit),
            Self::SettingsGet(m) => m.emit_parameters(xmit),
            Self::SettingsStatus(m) => m.emit_parameters(xmit),
            Self::SettingGet(m) => m.emit_parameters(xmit),
            Self::SettingSet(m) => m.emit_parameters(xmit),
            Self::SettingSetUnacknowledged(m) => m.emit_parameters(xmit),
            Self::SettingStatus(m) => m.emit_parameters(xmit),
        }
    }
}

impl<C, const NUM_SENSORS: usize, const NUM_COLUMNS: usize> Model
    for SensorServer<C, NUM_SENSORS, NUM_COLUMNS>
where
    C: SensorConfig,
{
    const IDENTIFIER: ModelIdentifier = SENSOR_SERVER;
    type Message = SensorMessage<C, NUM_SENSORS, NUM_COLUMNS>;

    fn parse(opcode: &Opcode, parameters: &[u8]) -> Result<Option<Self::Message>, ParseError> {
        match *opcode {
            SENSOR_DESCRIPTOR_GET => Ok(Some(SensorMessage::DescriptorGet(DescriptorGet::parse(
                parameters,
            )?))),
            SENSOR_GET => Ok(Some(SensorMessage::Get(SensorGet::parse(parameters)?))),
            SENSOR_COLUMN_GET => Ok(Some(SensorMessage::ColumnGet(ColumnGet::parse::<C>(
                parameters,
            )?))),
            SENSOR_SERIES_GET => Ok(Some(SensorMessage::SeriesGet(SeriesGet::parse::<C>(
                parameters,
            )?))),
            _ => Ok(None),
        }
    }
}

impl<C, const NUM_SENSORS: usize, const NUM_COLUMNS: usize> Model
    for SensorClient<C, NUM_SENSORS, NUM_COLUMNS>
where
    C: SensorConfig,
{
    const IDENTIFIER: ModelIdentifier = SENSOR_CLIENT;
    type Message = SensorMessage<C, NUM_SENSORS, NUM_COLUMNS>;

    fn parse(opcode: &Opcode, parameters: &[u8]) -> Result<Option<Self::Message>, ParseError> {
        match *opcode {
            SENSOR_DESCRIPTOR_GET => Ok(Some(SensorMessage::DescriptorGet(DescriptorGet::parse(
                parameters,
            )?))),
            SENSOR_GET => Ok(Some(SensorMessage::Get(SensorGet::parse(parameters)?))),
            SENSOR_STATUS => Ok(Some(SensorMessage::Status(SensorStatus::parse(
                parameters,
            )?))),
            SENSOR_COLUMN_GET => Ok(Some(SensorMessage::ColumnGet(ColumnGet::parse::<C>(
                parameters,
            )?))),
            SENSOR_SERIES_GET => Ok(Some(SensorMessage::SeriesGet(SeriesGet::parse::<C>(
                parameters,
            )?))),
            _ => Ok(None),
        }
    }
}

impl<C, const NUM_SENSORS: usize, const NUM_COLUMNS: usize> Model
    for SensorSetupServer<C, NUM_SENSORS, NUM_COLUMNS>
where
    C: SensorSetupConfig,
{
    const IDENTIFIER: ModelIdentifier = SENSOR_SETUP_SERVER;
    type Message = SensorMessage<C, NUM_SENSORS, NUM_COLUMNS>;

    fn parse(opcode: &Opcode, parameters: &[u8]) -> Result<Option<Self::Message>, ParseError> {
        match *opcode {
            SENSOR_DESCRIPTOR_GET => Ok(Some(SensorMessage::DescriptorGet(DescriptorGet::parse(
                parameters,
            )?))),
            SENSOR_GET => Ok(Some(SensorMessage::Get(SensorGet::parse(parameters)?))),
            SENSOR_STATUS => Ok(Some(SensorMessage::Status(SensorStatus::parse(
                parameters,
            )?))),
            SENSOR_COLUMN_GET => Ok(Some(SensorMessage::ColumnGet(ColumnGet::parse::<C>(
                parameters,
            )?))),
            SENSOR_SERIES_GET => Ok(Some(SensorMessage::SeriesGet(SeriesGet::parse::<C>(
                parameters,
            )?))),
            SENSOR_CADENCE_GET => Ok(Some(SensorMessage::CadenceGet(CadenceGet::parse(
                parameters,
            )?))),
            SENSOR_CADENCE_SET => Ok(Some(SensorMessage::CadenceSet(CadenceSet::parse::<C>(
                parameters,
            )?))),
            SENSOR_CADENCE_SET_UNACKNOWLEDGED => Ok(Some(SensorMessage::CadenceSetUnacknowledged(
                CadenceSet::parse::<C>(parameters)?,
            ))),
            SENSOR_SETTINGS_GET => Ok(Some(SensorMessage::SettingsGet(SettingsGet::parse(
                parameters,
            )?))),
            SENSOR_SETTING_GET => Ok(Some(SensorMessage::SettingGet(SettingGet::parse(
                parameters,
            )?))),
            SENSOR_SETTING_SET => Ok(Some(SensorMessage::SettingSet(SettingSet::parse::<C>(
                parameters,
            )?))),
            SENSOR_SETTING_SET_UNACKNOWLEDGED => Ok(Some(SensorMessage::SettingSetUnacknowledged(
                SettingSet::parse::<C>(parameters)?,
            ))),
            _ => Ok(None),
        }
    }
}

impl PropertyId {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        xmit.extend_from_slice(&self.0.to_le_bytes())
            .map_err(|_| InsufficientBuffer)?;
        Ok(())
    }

    fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() < 2 {
            Err(ParseError::InvalidLength)
        } else {
            Ok(Self(u16::from_le_bytes([data[0], data[1]])))
        }
    }
}

impl DescriptorGet {
    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        let id = if !parameters.is_empty() {
            Some(PropertyId::parse(parameters)?)
        } else {
            None
        };
        Ok(Self { id })
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        if let Some(id) = &self.id {
            id.emit_parameters(xmit)?;
        }
        Ok(())
    }
}

impl<const NUM_SENSORS: usize> DescriptorStatus<NUM_SENSORS> {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        match self {
            Self::NotFound(prop) => {
                prop.emit_parameters(xmit)?;
            }
            Self::Descriptors(descriptors) => {
                for d in descriptors {
                    d.emit_parameters(xmit)?;
                }
            }
        }
        Ok(())
    }
}

impl SensorDescriptor {
    pub const fn new(id: PropertyId, size: usize) -> Self {
        Self {
            id,
            positive_tolerance: Tolerance(0),
            negative_tolerance: Tolerance(0),
            sampling_function: SamplingFunction::Unspecified,
            measurement_period: None,
            update_interval: None,
            size,
            x_size: 0,
        }
    }
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        self.id.emit_parameters(xmit)?;

        let mut data: [u8; 3] = [0; 3];

        let d = self.positive_tolerance.0.to_le_bytes();
        data[0] = d[0];
        data[1] = d[1] << 4;

        let d = self.negative_tolerance.0.to_le_bytes();
        data[1] |= d[0];
        data[2] = d[1];

        xmit.extend_from_slice(&data)
            .map_err(|_| InsufficientBuffer)?;

        self.sampling_function.emit_parameters(xmit)?;

        let value = if let Some(m) = self.measurement_period {
            log_1_1(m as f32)
        } else {
            0
        };
        xmit.push(value).map_err(|_| InsufficientBuffer)?;

        let value = if let Some(m) = self.update_interval {
            log_1_1(m as f32)
        } else {
            0
        };
        xmit.push(value).map_err(|_| InsufficientBuffer)?;
        Ok(())
    }
}

impl SamplingFunction {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        match self {
            Self::Unspecified => xmit.push(0x00).map_err(|_| InsufficientBuffer)?,
            Self::Instantaneous => xmit.push(0x01).map_err(|_| InsufficientBuffer)?,
            Self::ArithmeticMean => xmit.push(0x02).map_err(|_| InsufficientBuffer)?,
            Self::RMS => xmit.push(0x03).map_err(|_| InsufficientBuffer)?,
            Self::Maximum => xmit.push(0x04).map_err(|_| InsufficientBuffer)?,
            Self::Minimum => xmit.push(0x05).map_err(|_| InsufficientBuffer)?,
            Self::Accumulated => xmit.push(0x06).map_err(|_| InsufficientBuffer)?,
            Self::Count => xmit.push(0x07).map_err(|_| InsufficientBuffer)?,
        }
        Ok(())
    }
}

impl SensorGet {
    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        let id = if !parameters.is_empty() {
            Some(PropertyId::parse(parameters)?)
        } else {
            None
        };
        Ok(Self { id })
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        if let Some(id) = &self.id {
            id.emit_parameters(xmit)?;
        }
        Ok(())
    }
}

impl<C> SensorStatus<C>
where
    C: SensorConfig,
{
    pub fn new(data: C::Data) -> Self {
        Self { data }
    }

    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        let mut data = C::Data::default();
        let mut pos = 0;
        for d in C::DESCRIPTORS {
            let format = parameters[pos] & 0b1000_0000;
            let (length, id, offset): (usize, u16, usize);

            if format == 0 {
                length = ((parameters[pos] & 0b0111_1000) >> 3) as usize;
                id = ((parameters[pos] & 0b0000_0111) | parameters[pos + 1]).into();
                offset = 2;
            } else {
                length = (parameters[pos] & 0b0111_1111) as usize;
                id = ((parameters[pos + 1] as u16) << 8) | parameters[pos + 2] as u16;
                offset = 3;
            }

            if id == d.id.0 && d.size == length {
                pos += offset;
            } else {
                return Err(ParseError::InvalidValue);
            }

            let parameters = &parameters[pos..(pos + d.size)];
            data.decode(d.id, parameters)?;
            pos += d.size;
        }
        Ok(Self { data })
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        let descriptors = C::DESCRIPTORS;
        for d in descriptors {
            self.emit_property(d.id, d.size, xmit)?;
        }
        Ok(())
    }

    fn emit_property<const N: usize>(
        &self,
        id: PropertyId,
        size: usize,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        let len = size;

        if id.0 < 2048 && len <= 16 {
            let value: u8 = (len as u8) << 3;
            let value: u8 = value | ((id.0 & 0x0700) >> 8) as u8;
            xmit.push(value).map_err(|_| InsufficientBuffer)?;

            let value = (id.0 & 0xFF) as u8;
            xmit.push(value).map_err(|_| InsufficientBuffer)?;
        } else {
            let value: u8 = 0x80;
            let value: u8 = value | len as u8 & 0x7F;
            xmit.push(value).map_err(|_| InsufficientBuffer)?;

            xmit.push((id.0 & 0xFF00 >> 8) as u8)
                .map_err(|_| InsufficientBuffer)?;
            xmit.push((id.0 & 0xFF) as u8)
                .map_err(|_| InsufficientBuffer)?;
        }

        self.data.encode(id, xmit).map_err(|_| InsufficientBuffer)?;
        Ok(())
    }
}

fn lookup_descriptor<C>(p: PropertyId) -> Option<&'static SensorDescriptor>
where
    C: SensorConfig,
{
    C::DESCRIPTORS.iter().find(|&d| d.id == p)
}

fn lookup_cadence_descriptor<C>(p: PropertyId) -> Option<&'static CadenceDescriptor>
where
    C: SensorSetupConfig,
{
    C::CADENCE_DESCRIPTORS.iter().find(|&d| d.id == p)
}

fn lookup_setting_descriptor<C>(p: PropertyId, s: PropertyId) -> Option<&'static SettingDescriptor>
where
    C: SensorSetupConfig,
{
    C::SETTING_DESCRIPTORS
        .iter()
        .find(|&d| d.sensor == p && d.setting == s)
}

impl ColumnGet {
    fn parse<C>(parameters: &[u8]) -> Result<Self, ParseError>
    where
        C: SensorConfig,
    {
        let id = PropertyId::parse(parameters)?;

        if let Some(d) = lookup_descriptor::<C>(id) {
            let x_len = d.x_size;
            let parameters = &parameters[2..];
            Ok(Self {
                id,
                x: RawValue(Vec::from_slice(&parameters[..x_len])?),
            })
        } else {
            Err(ParseError::InvalidValue)
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        self.id.emit_parameters(xmit)?;
        self.x.emit_parameters(xmit)?;
        Ok(())
    }
}

impl ColumnStatus {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        self.id.emit_parameters(xmit)?;
        self.x.emit_parameters(xmit)?;
        if let Some((w, y)) = &self.values {
            w.emit_parameters(xmit)?;
            y.emit_parameters(xmit)?;
        }
        Ok(())
    }
}

impl SeriesGet {
    fn parse<C>(parameters: &[u8]) -> Result<Self, ParseError>
    where
        C: SensorConfig,
    {
        let id = PropertyId::parse(parameters)?;
        let parameters = &parameters[2..];
        if !parameters.is_empty() {
            if let Some(d) = lookup_descriptor::<C>(id) {
                let x_len = d.x_size;
                let x1 = RawValue(Vec::from_slice(&parameters[..x_len])?);
                let parameters = &parameters[x_len..];
                let x2 = RawValue(Vec::from_slice(&parameters[..x_len])?);
                Ok(Self {
                    id,
                    x: Some((x1, x2)),
                })
            } else {
                Err(ParseError::InvalidValue)
            }
        } else {
            Ok(Self { id, x: None })
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        self.id.emit_parameters(xmit)?;
        if let Some((x1, x2)) = &self.x {
            x1.emit_parameters(xmit)?;
            x2.emit_parameters(xmit)?;
        }
        Ok(())
    }
}

impl<const NUM_COLUMNS: usize> SeriesStatus<NUM_COLUMNS> {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        self.id.emit_parameters(xmit)?;
        for (x, w, y) in self.values.iter() {
            x.emit_parameters(xmit)?;
            w.emit_parameters(xmit)?;
            y.emit_parameters(xmit)?;
        }
        Ok(())
    }
}

impl RawValue {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        xmit.extend_from_slice(&self.0)
            .map_err(|_| InsufficientBuffer)?;
        Ok(())
    }
}

impl CadenceGet {
    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        let id = PropertyId::parse(parameters)?;
        Ok(Self { id })
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        self.id.emit_parameters(xmit)?;
        Ok(())
    }
}

impl CadenceSet {
    fn parse<C>(parameters: &[u8]) -> Result<Self, ParseError>
    where
        C: SensorSetupConfig,
    {
        let id = PropertyId::parse(parameters)?;
        let fast_cadence_divisor = parameters[2] & 0xF7 >> 1;
        let status_trigger_type = if parameters[2] & 0x01 == 1 {
            StatusTriggerType::Unitless
        } else {
            StatusTriggerType::Property
        };

        if let Some(d) = lookup_cadence_descriptor::<C>(id) {
            let c_len = d.size;

            let parameters = &parameters[3..];
            let status_trigger_delta_down = RawValue(Vec::from_slice(&parameters[..c_len])?);
            let parameters = &parameters[c_len..];
            let status_trigger_delta_up = RawValue(Vec::from_slice(&parameters[..c_len])?);
            let parameters = &parameters[c_len..];

            let status_min_interval = parameters[0];
            let parameters = &parameters[1..];

            let fast_cadence_low = RawValue(Vec::from_slice(&parameters[..c_len])?);
            let parameters = &parameters[c_len..];
            let fast_cadence_high = RawValue(Vec::from_slice(&parameters[..c_len])?);

            Ok(Self {
                id,
                fast_cadence_divisor,
                status_trigger_type,
                status_trigger_delta_down,
                status_trigger_delta_up,
                status_min_interval,
                fast_cadence_low,
                fast_cadence_high,
            })
        } else {
            Err(ParseError::InvalidValue)
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        self.id.emit_parameters(xmit)?;
        let val = self.fast_cadence_divisor << 1;
        let val = val
            | match self.status_trigger_type {
                StatusTriggerType::Unitless => 1,
                _ => 0,
            };
        xmit.push(val).map_err(|_| InsufficientBuffer)?;
        self.status_trigger_delta_down.emit_parameters(xmit)?;
        self.status_trigger_delta_up.emit_parameters(xmit)?;
        xmit.push(self.status_min_interval)
            .map_err(|_| InsufficientBuffer)?;

        self.fast_cadence_low.emit_parameters(xmit)?;
        self.fast_cadence_high.emit_parameters(xmit)?;
        Ok(())
    }
}

impl SettingsGet {
    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        let id = PropertyId::parse(parameters)?;
        Ok(Self { id })
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        self.id.emit_parameters(xmit)?;
        Ok(())
    }
}

impl<const NUM_SENSORS: usize> SettingsStatus<NUM_SENSORS> {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        self.id.emit_parameters(xmit)?;
        for setting in self.settings.iter() {
            setting.emit_parameters(xmit)?;
        }
        Ok(())
    }
}

impl SettingGet {
    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        let id = PropertyId::parse(parameters)?;
        let setting = PropertyId::parse(&parameters[2..])?;

        Ok(Self { id, setting })
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        self.id.emit_parameters(xmit)?;
        self.setting.emit_parameters(xmit)?;
        Ok(())
    }
}

impl SettingSet {
    fn parse<C>(parameters: &[u8]) -> Result<Self, ParseError>
    where
        C: SensorSetupConfig,
    {
        let id = PropertyId::parse(parameters)?;
        let setting = PropertyId::parse(&parameters[2..])?;

        if let Some(d) = lookup_setting_descriptor::<C>(id, setting) {
            let s_len = d.size;
            let raw = RawValue(Vec::from_slice(&parameters[4..4 + s_len])?);

            Ok(Self { id, setting, raw })
        } else {
            Err(ParseError::InvalidValue)
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        self.id.emit_parameters(xmit)?;
        self.setting.emit_parameters(xmit)?;
        self.raw.emit_parameters(xmit)?;
        Ok(())
    }
}

impl SettingStatus {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut heapless::Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        self.id.emit_parameters(xmit)?;
        self.setting.emit_parameters(xmit)?;
        xmit.push(match self.access {
            SensorSettingAccess::Read => 1,
            SensorSettingAccess::ReadWrite => 3,
        })
        .map_err(|_| InsufficientBuffer)?;
        self.raw.emit_parameters(xmit)?;
        Ok(())
    }
}

/// Approxmiates the log with base 1.1
fn log_1_1(seconds: f32) -> u8 {
    (seconds.log(1.1) as u8) + 64
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROP_TEMP: PropertyId = PropertyId(0x4F);
    const PROP_MOTION: PropertyId = PropertyId(0x42);

    #[derive(Debug)]
    pub struct SensorPayload {
        pub temperature: i8,
        pub motion: u8,
    }

    impl Default for SensorPayload {
        fn default() -> Self {
            Self {
                temperature: 0,
                motion: 0,
            }
        }
    }

    impl SensorData for SensorPayload {
        fn decode(&mut self, id: PropertyId, params: &[u8]) -> Result<(), ParseError> {
            if id == PROP_TEMP {
                self.temperature = params[0] as i8;
                Ok(())
            } else if id == PROP_MOTION {
                self.motion = params[0];
                Ok(())
            } else {
                Err(ParseError::InvalidValue)
            }
        }

        fn encode<const N: usize>(
            &self,
            property: PropertyId,
            xmit: &mut Vec<u8, N>,
        ) -> Result<(), InsufficientBuffer> {
            if property == PROP_TEMP {
                xmit.extend_from_slice(&self.temperature.to_le_bytes())
                    .map_err(|_| InsufficientBuffer)?;
            } else if property == PROP_MOTION {
                xmit.extend_from_slice(&self.motion.to_le_bytes())
                    .map_err(|_| InsufficientBuffer)?;
            }
            Ok(())
        }
    }

    #[derive(Clone, Debug)]
    pub struct TestSensor;

    impl SensorConfig for TestSensor {
        type Data = SensorPayload;

        const DESCRIPTORS: &'static [SensorDescriptor] = &[
            SensorDescriptor::new(PROP_TEMP, 1),
            SensorDescriptor::new(PROP_MOTION, 1),
        ];
    }

    #[test]
    fn test_sensor_codec() {
        let data = SensorPayload::default();
        let msg: SensorMessage<TestSensor, 2, 1> = SensorMessage::Status(SensorStatus::new(data));
        let mut parameters: heapless::Vec<u8, 386> = heapless::Vec::new();
        msg.emit_parameters(&mut parameters).unwrap();

        let _parsed: SensorMessage<TestSensor, 2, 1> =
            SensorClient::parse(&msg.opcode(), &parameters[..])
                .unwrap()
                .unwrap();
    }
}

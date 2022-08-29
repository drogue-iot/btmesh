use crate::{Message, Model};
use btmesh_common::opcode::Opcode;
use btmesh_common::{opcode, InsufficientBuffer, ModelIdentifier, ParseError};
use heapless::Vec;

#[derive(Clone, Debug)]
pub struct GenericOnOffServer;

#[derive(Clone, Debug)]
pub struct GenericOnOffClient;

pub const GENERIC_ONOFF_SERVER: ModelIdentifier = ModelIdentifier::SIG(0x1000);
pub const GENERIC_ONOFF_CLIENT: ModelIdentifier = ModelIdentifier::SIG(0x1001);

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GenericOnOffMessage {
    Get,
    Set(Set),
    SetUnacknowledged(Set),
    Status(Status),
}

impl Message for GenericOnOffMessage {
    fn opcode(&self) -> Opcode {
        match self {
            GenericOnOffMessage::Get => GENERIC_ON_OFF_GET,
            GenericOnOffMessage::Set(_) => GENERIC_ON_OFF_SET,
            GenericOnOffMessage::SetUnacknowledged(_) => GENERIC_ON_OFF_SET_UNACKNOWLEDGE,
            GenericOnOffMessage::Status(_) => GENERIC_ON_OFF_STATUS,
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        match self {
            GenericOnOffMessage::Get => Ok(()),
            GenericOnOffMessage::Set(inner) => inner.emit_parameters(xmit),
            GenericOnOffMessage::SetUnacknowledged(inner) => inner.emit_parameters(xmit),
            GenericOnOffMessage::Status(inner) => inner.emit_parameters(xmit),
        }
    }
}

impl Model for GenericOnOffServer {
    const IDENTIFIER: ModelIdentifier = GENERIC_ONOFF_SERVER;
    type Message = GenericOnOffMessage;

    fn parse(opcode: &Opcode, parameters: &[u8]) -> Result<Option<Self::Message>, ParseError> {
        match *opcode {
            GENERIC_ON_OFF_GET => Ok(None),
            GENERIC_ON_OFF_SET => Ok(Some(GenericOnOffMessage::Set(Set::parse(parameters)?))),
            GENERIC_ON_OFF_SET_UNACKNOWLEDGE => {
                Ok(Some(GenericOnOffMessage::Set(Set::parse(parameters)?)))
            }
            _ => {
                // not applicable to this role
                Ok(None)
            }
        }
    }
}

impl Model for GenericOnOffClient {
    const IDENTIFIER: ModelIdentifier = GENERIC_ONOFF_CLIENT;
    type Message = GenericOnOffMessage;

    fn parse(opcode: &Opcode, parameters: &[u8]) -> Result<Option<Self::Message>, ParseError> {
        match *opcode {
            GENERIC_ON_OFF_STATUS => Ok(Some(GenericOnOffMessage::Status(Status::parse(
                parameters,
            )?))),
            _ => {
                // not applicable to this role
                Ok(None)
            }
        }
    }
}

opcode!( GENERIC_ON_OFF_GET 0x82, 0x01 );
opcode!( GENERIC_ON_OFF_SET 0x82, 0x02 );
opcode!( GENERIC_ON_OFF_SET_UNACKNOWLEDGE 0x82, 0x03 );
opcode!( GENERIC_ON_OFF_STATUS 0x82, 0x04 );

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Set {
    pub on_off: u8,
    pub tid: u8,
    pub transition_time: Option<u8>,
    pub delay: Option<u8>,
}

impl Set {
    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 2 {
            let on_off = parameters[0];
            let tid = parameters[1];
            let transition_time = if parameters.len() >= 3 {
                Some(parameters[2])
            } else {
                None
            };
            let delay = if parameters.len() >= 4 {
                Some(parameters[3])
            } else {
                None
            };

            Ok(Self {
                on_off,
                tid,
                transition_time,
                delay,
            })
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        xmit.push(self.on_off).map_err(|_| InsufficientBuffer)?;
        xmit.push(self.tid).map_err(|_| InsufficientBuffer)?;
        if let Some(transition_time) = self.transition_time {
            xmit.push(transition_time).map_err(|_| InsufficientBuffer)?;
            if let Some(delay) = self.delay {
                xmit.push(delay).map_err(|_| InsufficientBuffer)?;
            }
        }
        Ok(())
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Status {
    pub present_on_off: u8,
    pub target_on_off: u8,
    pub remaining_time: u8,
}

impl From<Status> for GenericOnOffMessage {
    fn from(inner: Status) -> Self {
        GenericOnOffMessage::Status(inner)
    }
}

impl Status {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        xmit.push(self.present_on_off)
            .map_err(|_| InsufficientBuffer)?;
        xmit.push(self.target_on_off)
            .map_err(|_| InsufficientBuffer)?;
        xmit.push(self.remaining_time)
            .map_err(|_| InsufficientBuffer)?;
        Ok(())
    }

    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 3 {
            let present_on_off = parameters[0];
            let target_on_off = parameters[1];
            let remaining_time = parameters[2];
            Ok(Self {
                present_on_off,
                target_on_off,
                remaining_time,
            })
        } else {
            Err(ParseError::InvalidLength)
        }
    }
}

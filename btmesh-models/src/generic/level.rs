use crate::{Message, Model};
use btmesh_common::opcode::Opcode;
use btmesh_common::{opcode, InsufficientBuffer, ModelIdentifier, ParseError};
use heapless::Vec;

#[derive(Clone, Debug)]
pub struct GenericLevelServer;

#[derive(Clone, Debug)]
pub struct GenericLevelClient;

pub const GENERIC_LEVEL_SERVER: ModelIdentifier = ModelIdentifier::SIG(0x1002);
pub const GENERIC_LEVEL_CLIENT: ModelIdentifier = ModelIdentifier::SIG(0x1003);

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GenericLevelMessage {
    Get,
    Set(GenericLevelSet),
    SetUnacknowledged(GenericLevelSet),
    DeltaSet(GenericDeltaSet),
    DeltaSetUnacknowledged(GenericDeltaSet),
    MoveSet(GenericMoveSet),
    MoveSetUnacknowledged(GenericMoveSet),
    Status(GenericLevelStatus),
}

impl Message for GenericLevelMessage {
    fn opcode(&self) -> Opcode {
        match self {
            GenericLevelMessage::Get => GENERIC_LEVEL_GET,
            GenericLevelMessage::Set(_) => GENERIC_LEVEL_SET,
            GenericLevelMessage::SetUnacknowledged(_) => GENERIC_LEVEL_SET_UNACKNOWLEDGED,
            GenericLevelMessage::DeltaSet(_) => GENERIC_LEVEL_DELTA_SET,
            GenericLevelMessage::DeltaSetUnacknowledged(_) => {
                GENERIC_LEVEL_DELTA_SET_UNACKNOWLEDGED
            }
            GenericLevelMessage::MoveSet(_) => GENERIC_LEVEL_MOVE_SET,
            GenericLevelMessage::MoveSetUnacknowledged(_) => GENERIC_LEVEL_MOVE_SET_UNACKNOWLEDGED,
            GenericLevelMessage::Status(_) => GENERIC_LEVEL_STATUS,
        }
    }

    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        match self {
            GenericLevelMessage::Get => Ok(()),
            GenericLevelMessage::Set(inner) => inner.emit_parameters(xmit),
            GenericLevelMessage::SetUnacknowledged(inner) => inner.emit_parameters(xmit),
            GenericLevelMessage::DeltaSet(inner) => inner.emit_parameters(xmit),
            GenericLevelMessage::DeltaSetUnacknowledged(inner) => inner.emit_parameters(xmit),
            GenericLevelMessage::MoveSet(inner) => inner.emit_parameters(xmit),
            GenericLevelMessage::MoveSetUnacknowledged(inner) => inner.emit_parameters(xmit),
            GenericLevelMessage::Status(inner) => inner.emit_parameters(xmit),
        }
    }
}

impl GenericLevelMessage {
    fn parse(opcode: &Opcode, parameters: &[u8]) -> Result<Option<Self>, ParseError> {
        match *opcode {
            GENERIC_LEVEL_GET => Ok(Some(GenericLevelMessage::Get)),
            GENERIC_LEVEL_SET => Ok(Some(GenericLevelMessage::Set(GenericLevelSet::parse(
                parameters,
            )?))),
            GENERIC_LEVEL_SET_UNACKNOWLEDGED => Ok(Some(GenericLevelMessage::SetUnacknowledged(
                GenericLevelSet::parse(parameters)?,
            ))),
            GENERIC_LEVEL_DELTA_SET => Ok(Some(GenericLevelMessage::DeltaSet(
                GenericDeltaSet::parse(parameters)?,
            ))),
            GENERIC_LEVEL_DELTA_SET_UNACKNOWLEDGED => Ok(Some(
                GenericLevelMessage::DeltaSetUnacknowledged(GenericDeltaSet::parse(parameters)?),
            )),
            GENERIC_LEVEL_MOVE_SET => Ok(Some(GenericLevelMessage::MoveSet(
                GenericMoveSet::parse(parameters)?,
            ))),
            GENERIC_LEVEL_MOVE_SET_UNACKNOWLEDGED => Ok(Some(
                GenericLevelMessage::MoveSetUnacknowledged(GenericMoveSet::parse(parameters)?),
            )),
            GENERIC_LEVEL_STATUS => Ok(Some(GenericLevelMessage::Status(
                GenericLevelStatus::parse(parameters)?,
            ))),
            _ => {
                // not applicable to this role
                Ok(None)
            }
        }
    }
}

impl Model for GenericLevelServer {
    const IDENTIFIER: ModelIdentifier = GENERIC_LEVEL_SERVER;
    type Message = GenericLevelMessage;

    fn parse(opcode: &Opcode, parameters: &[u8]) -> Result<Option<Self::Message>, ParseError> {
        GenericLevelMessage::parse(opcode, parameters)
    }
}

impl Model for GenericLevelClient {
    const IDENTIFIER: ModelIdentifier = GENERIC_LEVEL_CLIENT;
    type Message = GenericLevelMessage;

    fn parse(opcode: &Opcode, parameters: &[u8]) -> Result<Option<Self::Message>, ParseError> {
        GenericLevelMessage::parse(opcode, parameters)
    }
}

opcode!( GENERIC_LEVEL_GET 0x82, 0x05 );
opcode!( GENERIC_LEVEL_SET 0x82, 0x06 );
opcode!( GENERIC_LEVEL_SET_UNACKNOWLEDGED 0x82, 0x07 );
opcode!( GENERIC_LEVEL_STATUS 0x82, 0x08 );
opcode!( GENERIC_LEVEL_DELTA_SET 0x82, 0x09 );
opcode!( GENERIC_LEVEL_DELTA_SET_UNACKNOWLEDGED 0x82, 0x0A );
opcode!( GENERIC_LEVEL_MOVE_SET 0x82, 0x0B );
opcode!( GENERIC_LEVEL_MOVE_SET_UNACKNOWLEDGED 0x82, 0x0C );

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GenericLevelSet {
    pub level: i16,
    pub tid: u8,
    pub transition_time: Option<u8>,
    pub delay: Option<u8>,
}

impl GenericLevelSet {
    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 3 {
            let level: i16 = i16::from_le_bytes([parameters[0], parameters[1]]);
            let tid = parameters[2];
            let transition_time = if parameters.len() >= 4 {
                Some(parameters[3])
            } else {
                None
            };
            let delay = if parameters.len() >= 5 {
                Some(parameters[4])
            } else {
                None
            };

            Ok(Self {
                level,
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
        xmit.extend_from_slice(&self.level.to_le_bytes()[..])
            .map_err(|_| InsufficientBuffer)?;
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
pub struct GenericDeltaSet {
    pub delta_level: i32,
    pub tid: u8,
    pub transition_time: Option<u8>,
    pub delay: Option<u8>,
}

impl GenericDeltaSet {
    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 5 {
            let delta_level: i32 =
                i32::from_le_bytes([parameters[0], parameters[1], parameters[2], parameters[3]]);
            let tid = parameters[4];
            let transition_time = if parameters.len() >= 6 {
                Some(parameters[5])
            } else {
                None
            };
            let delay = if parameters.len() >= 7 {
                Some(parameters[6])
            } else {
                None
            };

            Ok(Self {
                delta_level,
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
        xmit.extend_from_slice(&self.delta_level.to_le_bytes()[..])
            .map_err(|_| InsufficientBuffer)?;
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
pub struct GenericMoveSet {
    pub delta_level: i16,
    pub tid: u8,
    pub transition_time: Option<u8>,
    pub delay: Option<u8>,
}

impl GenericMoveSet {
    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 3 {
            let delta_level: i16 = i16::from_le_bytes([parameters[0], parameters[1]]);
            let tid = parameters[2];
            let transition_time = if parameters.len() >= 4 {
                Some(parameters[3])
            } else {
                None
            };
            let delay = if parameters.len() >= 5 {
                Some(parameters[4])
            } else {
                None
            };

            Ok(Self {
                delta_level,
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
        xmit.extend_from_slice(&self.delta_level.to_le_bytes()[..])
            .map_err(|_| InsufficientBuffer)?;
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
pub struct GenericLevelStatus {
    pub present_level: i16,
    pub target_level: Option<i16>,
    pub remaining_time: Option<u8>,
}

impl From<GenericLevelStatus> for GenericLevelMessage {
    fn from(inner: GenericLevelStatus) -> Self {
        GenericLevelMessage::Status(inner)
    }
}

impl GenericLevelStatus {
    fn emit_parameters<const N: usize>(
        &self,
        xmit: &mut Vec<u8, N>,
    ) -> Result<(), InsufficientBuffer> {
        xmit.extend_from_slice(&self.present_level.to_le_bytes()[..])
            .map_err(|_| InsufficientBuffer)?;

        if let Some(target_level) = self.target_level {
            xmit.extend_from_slice(&target_level.to_le_bytes()[..])
                .map_err(|_| InsufficientBuffer)?;
        }

        if let Some(remaining_time) = self.remaining_time {
            xmit.push(remaining_time).map_err(|_| InsufficientBuffer)?;
        }
        Ok(())
    }

    fn parse(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.len() >= 2 {
            let present_level: i16 = i16::from_le_bytes([parameters[0], parameters[1]]);
            let target_level = if parameters.len() >= 4 {
                let target_level: i16 = i16::from_le_bytes([parameters[2], parameters[3]]);
                Some(target_level)
            } else {
                None
            };
            let remaining_time = if parameters.len() >= 5 {
                Some(parameters[4])
            } else {
                None
            };
            Ok(Self {
                present_level,
                target_level,
                remaining_time,
            })
        } else {
            Err(ParseError::InvalidLength)
        }
    }
}

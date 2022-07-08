use crate::stack::unprovisioned::UnprovisionedStack;
use crate::{DeviceState, ProvisionedStack, Sequence};
use btmesh_common::Uuid;
use embassy::time::Instant;

pub mod interface;
pub mod provisioned;
pub mod unprovisioned;

#[allow(clippy::large_enum_variant)]
pub enum Stack {
    Unprovisioned {
        stack: UnprovisionedStack,
        num_elements: u8,
        uuid: Uuid,
    },
    Provisioned {
        stack: ProvisionedStack,
        sequence: Sequence,
    },
}

impl Stack {
    pub fn device_state(&self) -> DeviceState {
        match self {
            Stack::Unprovisioned { stack, uuid, .. } => DeviceState::Unprovisioned {
                uuid: *uuid,
                in_progress: stack.in_progress(),
            },
            Stack::Provisioned { .. } => DeviceState::Provisioned,
        }
    }

    pub fn next_beacon_deadline(&self) -> Option<Instant> {
        match self {
            Stack::Unprovisioned { stack, .. } => stack.next_beacon_deadline(),
            Stack::Provisioned { stack, .. } => stack.next_beacon_deadline(),
        }
    }
}

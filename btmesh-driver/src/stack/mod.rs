use crate::unprovisioned::UnprovisionedStack;
use crate::{DeviceState, ProvisionedStack, Sequence};

pub mod interface;
pub mod provisioned;

pub enum Stack {
    Unprovisioned(UnprovisionedStack),
    Provisioned(ProvisionedStack, Sequence),
}

impl Stack {
    pub fn device_state(&self) -> DeviceState {
        match self {
            Stack::Unprovisioned(stack) => DeviceState::Unprovisioned { uuid: todo!() },
            Stack::Provisioned(_, _) => DeviceState::Provisioned,
        }
    }
}

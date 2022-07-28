use crate::{Configuration, DeviceInfo, NetworkState, Secrets};
use core::hash::{Hash, Hasher};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct ProvisionedConfiguration {
    pub(crate) network_state: NetworkState,
    pub(crate) secrets: Secrets,
    pub(crate) device_info: DeviceInfo,
    pub(crate) sequence: u32,
}

impl ProvisionedConfiguration {
    pub fn network_state(&self) -> NetworkState {
        self.network_state
    }

    pub fn secrets(&self) -> Secrets {
        self.secrets.clone()
    }

    pub fn device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }

    pub fn sequence(&self) -> u32 {
        self.sequence
    }
}

impl Hash for ProvisionedConfiguration {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.network_state.hash(state);
        self.secrets.hash(state);
        self.device_info.hash(state);
        // explicitly skip sequence, checked separately.
    }
}

impl From<ProvisionedConfiguration> for Configuration {
    fn from(inner: ProvisionedConfiguration) -> Self {
        Configuration::Provisioned(inner)
    }
}

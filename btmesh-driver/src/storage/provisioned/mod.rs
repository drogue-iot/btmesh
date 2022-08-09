use crate::{Configuration, DeviceInfo, NetworkState, Secrets};
use core::hash::{Hash, Hasher};
use crate::storage::provisioned::foundation::Foundation;

mod foundation;

#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct ProvisionedConfiguration {
    pub(crate) network_state: NetworkState,
    pub(crate) secrets: Secrets,
    pub(crate) device_info: DeviceInfo,
    pub(crate) sequence: u32,
    pub(crate) foundation: Foundation,
}

impl ProvisionedConfiguration {
    pub fn network_state(&self) -> NetworkState {
        self.network_state
    }

    pub fn secrets(&self) -> Secrets {
        self.secrets.clone()
    }

    pub fn device_info(&self) -> DeviceInfo {
        self.device_info
    }

    pub fn sequence(&self) -> u32 {
        self.sequence
    }

    pub fn foundation(&self) -> &Foundation {
        &self.foundation
    }

    pub fn foundation_mut(&mut self) -> &mut Foundation {
        &mut self.foundation
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

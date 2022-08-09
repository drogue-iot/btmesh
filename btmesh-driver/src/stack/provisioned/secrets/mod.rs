use crate::stack::provisioned::secrets::application::ApplicationKeys;
use crate::stack::provisioned::secrets::network::NetworkKeys;
use crate::stack::provisioned::DriverError;
use btmesh_common::crypto::application::{Aid, ApplicationKey};
use btmesh_common::crypto::device::DeviceKey;
use btmesh_common::crypto::network::{NetworkKey, Nid};
use btmesh_pdu::provisioning::ProvisioningData;

use btmesh_device::{ApplicationKeyHandle, NetworkKeyHandle};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub mod application;
pub mod network;

#[derive(Clone, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Secrets {
    device_key: DeviceKey,
    network_keys: NetworkKeys,
    application_keys: ApplicationKeys,
}

impl From<(DeviceKey, ProvisioningData)> for Secrets {
    fn from(data: (DeviceKey, ProvisioningData)) -> Self {
        Self {
            device_key: data.0,
            network_keys: data.1.into(),
            application_keys: Default::default(),
        }
    }
}

impl Secrets {
    pub fn display(&self) {
        info!("device_key: {}", self.device_key);
        self.network_keys.display();
    }

    pub(crate) fn new(
        device_key: DeviceKey,
        network_keys: NetworkKeys,
        application_keys: ApplicationKeys,
    ) -> Self {
        Self {
            device_key,
            network_keys,
            application_keys,
        }
    }

    pub(crate) fn device_key(&self) -> DeviceKey {
        self.device_key
    }

    pub(crate) fn network_keys_by_nid(
        &self,
        nid: Nid,
    ) -> impl Iterator<Item = NetworkKeyHandle> + '_ {
        self.network_keys.by_nid_iter(nid)
    }

    pub(crate) fn network_key(
        &self,
        network_key: NetworkKeyHandle,
    ) -> Result<NetworkKey, DriverError> {
        self.network_keys.keys[network_key.0 as usize]
            .as_ref()
            .ok_or(DriverError::InvalidKeyHandle)
            .cloned()
    }

    pub(crate) fn network_key_by_index(&self, index: u8) -> Result<NetworkKey, DriverError> {
        if let Some(network_key) = self.network_keys.keys[index as usize] {
            Ok(network_key)
        } else {
            Err(DriverError::InvalidKeyHandle)
        }
    }

    pub(crate) fn application_keys_by_aid(
        &self,
        aid: Aid,
    ) -> impl Iterator<Item = ApplicationKeyHandle> + '_ {
        self.application_keys.by_aid_iter(aid)
    }

    pub(crate) fn application_key(
        &self,
        application_key: ApplicationKeyHandle,
    ) -> Result<ApplicationKey, DriverError> {
        self.application_keys.keys[application_key.0 as usize]
            .as_ref()
            .ok_or(DriverError::InvalidKeyHandle)
            .cloned()
    }
}

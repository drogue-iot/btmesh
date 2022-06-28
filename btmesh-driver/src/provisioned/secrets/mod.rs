use crate::provisioned::secrets::application::ApplicationKeys;
use crate::provisioned::secrets::network::NetworkKeys;
use crate::provisioned::system::{ApplicationKeyHandle, NetworkKeyHandle};
use crate::provisioned::DriverError;
use btmesh_common::crypto::application::{Aid, ApplicationKey};
use btmesh_common::crypto::device::DeviceKey;
use btmesh_common::crypto::network::{NetworkKey, Nid};

pub mod application;
pub mod network;

pub(crate) struct Secrets {
    device_key: DeviceKey,
    network_keys: NetworkKeys,
    application_keys: ApplicationKeys,
}

impl Secrets {
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

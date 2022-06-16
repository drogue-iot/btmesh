use crate::{ApplicationKeyHandle, DriverError, NetworkKeyHandle};
use crate::secrets::network::NetworkKeys;
use btmesh_common::{Aid, crypto, Nid};
use crate::secrets::application::ApplicationKeys;
use crate::secrets::device::DeviceKey;

pub mod device;
pub mod network;
pub mod application;

pub(crate) struct Secrets {
    device_key: DeviceKey,
    network_keys: NetworkKeys,
    application_keys: ApplicationKeys,
}

impl Secrets {
    pub(crate) fn device_key(&self) -> [u8;16] {
        self.device_key.device_key()
    }

    pub(crate) fn network_keys_by_nid(
        &self,
        nid: Nid,
    ) -> impl Iterator<Item = NetworkKeyHandle> + '_ {
        self.network_keys.by_nid_iter(nid)
    }

    pub(crate) fn network_privacy_key(
        &self,
        network_key: NetworkKeyHandle,
    ) -> Result<[u8; 16], DriverError> {
        self.network_keys.keys[network_key.0 as usize]
            .as_ref()
            .ok_or(DriverError::InvalidKeyHandle)
            .map(|key| key.privacy_key())
    }

    pub(crate) fn network_encryption_key(
        &self,
        network_key: NetworkKeyHandle,
    ) -> Result<[u8; 16], DriverError> {
        self.network_keys.keys[network_key.0 as usize]
            .as_ref()
            .ok_or(DriverError::InvalidKeyHandle)
            .map(|key| key.encryption_key())
    }

    pub(crate) fn application_keys_by_aid(
        &self,
        aid: Aid
    ) -> impl Iterator<Item = ApplicationKeyHandle> + '_ {
        self.application_keys.by_aid_iter(aid)
    }

    pub(crate) fn application_key(&self, application_key: ApplicationKeyHandle) -> Result<[u8; 16], DriverError> {
        self.application_keys.keys[application_key.0 as usize]
            .as_ref()
            .ok_or( DriverError::InvalidKeyHandle)
            .map(|key| key.application_key() )
    }
}

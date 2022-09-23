use crate::stack::provisioned::DriverError;
use btmesh_common::crypto::application::{Aid, ApplicationKey};
use heapless::Vec;

use btmesh_device::ApplicationKeyHandle;
use btmesh_models::foundation::configuration::{AppKeyIndex, NetKeyIndex};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
pub struct ApplicationKeys<const N: usize = 4> {
    keys: Vec<Option<(NetKeyIndex, ApplicationKey)>, N>,
}

impl<const N: usize> Default for ApplicationKeys<N> {
    fn default() -> Self {
        let mut keys = Vec::new();
        keys.resize(N, None).ok();
        Self { keys }
    }
}

impl<const N: usize> ApplicationKeys<N> {
    pub fn display(&self) {
        for (index, entry) in self.keys.iter().enumerate() {
            if let Some((net_key_index, app_key)) = entry {
                info!(
                    "application_key[{}]: {} (net_key_index: {})",
                    index, app_key, net_key_index
                );
            }
        }
    }

    pub(crate) fn has_key(&self, app_key_index: AppKeyIndex) -> bool {
        if usize::from(app_key_index) >= N {
            return false;
        }
        self.keys[usize::from(app_key_index)].is_some()
    }

    pub(crate) fn by_aid_iter(&self, aid: Aid) -> impl Iterator<Item = ApplicationKeyHandle> + '_ {
        self.keys
            .iter()
            .enumerate()
            .filter(move |e| {
                if let (_, Some((_, application_key))) = e {
                    application_key.aid() == aid
                } else {
                    false
                }
            })
            .map(move |(index, _)| ApplicationKeyHandle::new(AppKeyIndex::new(index as u16), aid))
    }

    pub(crate) fn get_key_details(
        &self,
        index: AppKeyIndex,
    ) -> Option<(NetKeyIndex, ApplicationKeyHandle)> {
        if let Some((net_key_index, app_key)) = self.keys[usize::from(index)] {
            Some((
                net_key_index,
                ApplicationKeyHandle::new(index, app_key.aid()),
            ))
        } else {
            None
        }
    }

    pub(crate) fn get<I: Into<AppKeyIndex>>(
        &self,
        index: I,
    ) -> Result<ApplicationKey, DriverError> {
        let index = index.into();
        debug!("get app-key {}", index);
        debug!(" --> {}", self.keys);
        if let Some(entry) = self.keys[usize::from(index)] {
            Ok(entry.1)
        } else {
            Err(DriverError::InvalidAppKeyIndex)
        }
    }

    pub fn add(
        &mut self,
        index: AppKeyIndex,
        net_key_index: NetKeyIndex,
        application_key: ApplicationKey,
    ) -> Result<(), DriverError> {
        if usize::from(index) >= N {
            return Err(DriverError::InvalidAppKeyIndex);
        }

        if let Some(..) = self.keys[usize::from(index)] {
            Err(DriverError::AppKeyIndexAlreadyStored)
        } else {
            self.keys[usize::from(index)].replace((net_key_index, application_key));
            Ok(())
        }
    }

    pub(crate) fn delete(
        &mut self,
        index: AppKeyIndex,
        net_key_index: NetKeyIndex,
    ) -> Result<(), DriverError> {
        if usize::from(index) >= N {
            return Err(DriverError::InvalidAppKeyIndex);
        }

        if let Some((current_net_key_index, ..)) = self.keys[usize::from(index)] {
            if net_key_index == current_net_key_index {
                self.keys[usize::from(index)].take();
                Ok(())
            } else {
                Err(DriverError::InvalidNetKeyIndex)
            }
        } else {
            Err(DriverError::InvalidAppKeyIndex)
        }
    }
}

use crate::stack::provisioned::system::ApplicationKeyHandle;
use crate::stack::provisioned::DriverError;
use btmesh_common::crypto::application::{Aid, ApplicationKey};
use heapless::Vec;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub(crate) struct ApplicationKeys<const N: usize = 4> {
    pub(crate) keys: Vec<Option<ApplicationKey>, N>,
}

impl<const N: usize> Default for ApplicationKeys<N> {
    fn default() -> Self {
        let mut keys = Vec::new();
        keys.resize(N, None).ok();
        Self { keys }
    }
}

impl<const N: usize> ApplicationKeys<N> {
    pub(crate) fn by_aid_iter(&self, aid: Aid) -> impl Iterator<Item = ApplicationKeyHandle> + '_ {
        self.keys
            .iter()
            .enumerate()
            .filter(move |e| {
                if let (_, Some(application_key)) = e {
                    application_key.aid() == aid
                } else {
                    false
                }
            })
            .map(move |(index, _)| ApplicationKeyHandle(index as u8, aid))
    }

    pub(crate) fn set(
        &mut self,
        index: u8,
        application_key: ApplicationKey,
    ) -> Result<(), DriverError> {
        if index as usize >= N {
            return Err(DriverError::InsufficientSpace);
        }

        self.keys[index as usize].replace(application_key);

        Ok(())
    }
}

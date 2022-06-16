use crate::{ApplicationKeyHandle, DriverError};
use btmesh_common::{crypto, Aid};

pub(crate) struct ApplicationKeys<const N: usize = 4> {
    pub(crate) keys: [Option<ApplicationKey>; N],
}

impl<const N: usize> Default for ApplicationKeys<N> {
    fn default() -> Self {
        let keys = [None; N];
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
                    application_key.aid == aid
                } else {
                    false
                }
            })
            .map(|(index, _)| ApplicationKeyHandle(index as u8))
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

#[derive(Copy, Clone)]
pub(crate) struct ApplicationKey {
    application_key: [u8; 16],
    aid: Aid,
}

impl ApplicationKey {
    pub fn new(application_key: [u8; 16]) -> Result<Self, DriverError> {
        let aid = crypto::k4(&application_key)
            .map_err(|_| DriverError::CryptoError)?
            .into();

        Ok(Self {
            application_key,
            aid,
        })
    }

    pub(crate) fn application_key(&self) -> [u8; 16] {
        self.application_key
    }
}

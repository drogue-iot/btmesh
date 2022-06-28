use crate::provisioned::system::ApplicationKeyHandle;
use crate::provisioned::DriverError;
use btmesh_common::Aid;
use btmesh_common::crypto::{
    self,
    application::ApplicationKey,
};

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



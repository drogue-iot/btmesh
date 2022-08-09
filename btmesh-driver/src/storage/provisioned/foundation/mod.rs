use crate::storage::provisioned::foundation::configuration::Configuration;

pub mod configuration;

#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct Foundation {
    configuration: Configuration,
}

impl Foundation {
    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }

    pub fn configuration_mut(&mut self) -> &mut Configuration {
        &mut self.configuration
    }
}

impl Default for Foundation {
    fn default() -> Self {
        Self {
            configuration: Default::default(),
        }
    }
}

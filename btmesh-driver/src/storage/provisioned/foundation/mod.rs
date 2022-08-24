use crate::storage::provisioned::foundation::configuration::Configuration;

pub mod configuration;

#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[derive(Clone, Debug, Hash, Default)]
pub struct Foundation {
    configuration: Configuration,
}

impl Foundation {
    pub fn display(&self) {
        info!("= foundation");
        self.configuration.display();
    }
    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }

    pub fn configuration_mut(&mut self) -> &mut Configuration {
        &mut self.configuration
    }
}

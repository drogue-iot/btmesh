use crate::DriverError;
use btmesh_common::{Composition, ModelIdentifier};
use btmesh_models::foundation::configuration::model_subscription::SubscriptionAddress;
use heapless::Vec;

#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Clone, Debug, Hash)]
pub struct Subscriptions<const N: usize = 64> {
    entries: Vec<Option<Subscription>, N>,
}

impl<const N: usize> Default for Subscriptions<N> {
    fn default() -> Self {
        let mut entries = Vec::new();
        entries.resize(N, None).ok();
        Self { entries }
    }
}

impl<const N: usize> Subscriptions<N> {

    pub fn display(&self, composition: &Composition) {
        info!("== subscriptions ==");
        for (index, element) in composition.elements_iter().enumerate() {
            info!("elements[{}]", index);
            for model_identifier in element.models_iter() {
                for address in  self.get(index as u8, *model_identifier) {
                    info!("  {} - subscription: {}", model_identifier, address);
                }
            }
        }
    }

    pub fn add(
        &mut self,
        composition: Composition,
        element_index: u8,
        model_identifier: ModelIdentifier,
        address: SubscriptionAddress,
    ) -> Result<(), DriverError> {
        if self.entries.iter().any(|e| {
            if let Some(slot) = e {
                slot.element_index == element_index
                    && slot.model_identifier == model_identifier
                    && slot.address == address
            } else {
                false
            }
        }) {
            return Ok(());
        }

        let descriptor = &composition[element_index];
        if descriptor.has_model(model_identifier) {
            if let Some(slot) = self.entries.iter_mut().find(|e| matches!(e, None)) {
                slot.replace(Subscription {
                    element_index,
                    model_identifier,
                    address,
                });
                Ok(())
            } else {
                Err(DriverError::InsufficientSpace)
            }
        } else {
            Err(DriverError::InvalidModel)
        }
    }

    pub fn delete(
        &mut self,
        element_index: u8,
        model_identifier: ModelIdentifier,
        address: SubscriptionAddress,
    ) -> Result<(), DriverError> {
        for matching in self.entries.iter_mut().filter( |e| {
            if let Some(slot) = e {
                slot.element_index == element_index
                    && slot.model_identifier == model_identifier
                    && slot.address == address
            } else {
                false
            }
        }) {
            matching.take();
        }
        Ok(())
    }

    pub fn delete_all(
        &mut self,
        element_index: u8,
        model_identifier: ModelIdentifier,
    ) -> Result<(), DriverError> {
        for matching in self.entries.iter_mut().filter( |e| {
            if let Some(slot) = e {
                slot.element_index == element_index
                    && slot.model_identifier == model_identifier
            } else {
                false
            }
        }) {
            matching.take();
        }
        Ok(())
    }

    pub fn get(&self, element_index: u8, model_identifier: ModelIdentifier) -> Vec<SubscriptionAddress, 12> {
        self.entries.iter().filter_map(|e| {
            if let Some(slot) = e {
                if slot.element_index == element_index && slot.model_identifier == model_identifier {
                    Some(slot.address)
                } else {
                    None
                }
            } else {
                None
            }
        }).collect()
    }
}

#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Clone, Debug, Hash)]
pub struct Subscription {
    element_index: u8,
    model_identifier: ModelIdentifier,
    address: SubscriptionAddress,
}

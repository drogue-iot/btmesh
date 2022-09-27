use crate::DriverError;
use btmesh_common::address::Address;
use btmesh_common::{Composition, ModelIdentifier};
use btmesh_models::foundation::configuration::model_subscription::SubscriptionAddress;
use heapless::Vec;

#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Clone, Debug, Hash)]
pub struct Subscriptions<const N: usize = 8> {
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
            for model_descriptor in element.models_iter() {
                for address in self.iter(index as u8, model_descriptor.model_identifier) {
                    info!("  {} <-- {}", model_descriptor.model_identifier, address);
                }
            }
        }
    }

    pub fn add(
        &mut self,
        composition: &Composition,
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
        for matching in self.entries.iter_mut().filter(|e| {
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
        for matching in self.entries.iter_mut().filter(|e| {
            if let Some(slot) = e {
                slot.element_index == element_index && slot.model_identifier == model_identifier
            } else {
                false
            }
        }) {
            matching.take();
        }
        Ok(())
    }

    pub fn get<const S: usize>(
        &self,
        element_index: u8,
        model_identifier: ModelIdentifier,
    ) -> Vec<SubscriptionAddress, S> {
        self.entries
            .iter()
            .filter_map(|e| {
                if let Some(slot) = e {
                    if slot.element_index == element_index
                        && slot.model_identifier == model_identifier
                    {
                        Some(slot.address)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn iter(
        &self,
        element_index: u8,
        model_identifier: ModelIdentifier,
    ) -> impl Iterator<Item = &Subscription> + '_ {
        self.entries.iter().filter_map(move |e| {
            if let Some(slot) = e {
                if slot.element_index == element_index && slot.model_identifier == model_identifier
                {
                    Some(slot)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    pub fn matches(&self, dst: Address) -> bool {
        if let Ok(subscription_dst) = dst.try_into() {
            self.entries
                .iter()
                .filter(move |e| {
                    if let Some(slot) = e {
                        slot.address == subscription_dst
                    } else {
                        false
                    }
                })
                .flatten()
                .count()
                > 0
        } else {
            false
        }
    }

    pub fn subscriptions_for(
        &self,
        dst: Address,
    ) -> Result<impl Iterator<Item = &Subscription> + '_, DriverError> {
        if let Ok(subscription_dst) = dst.try_into() {
            Ok(self
                .entries
                .iter()
                .filter(move |e| {
                    if let Some(slot) = e {
                        slot.address == subscription_dst
                    } else {
                        false
                    }
                })
                .flatten())
        } else {
            Err(DriverError::InvalidAddress)
        }
    }
}

#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Clone, Debug, Hash)]
pub struct Subscription {
    pub element_index: u8,
    pub model_identifier: ModelIdentifier,
    pub address: SubscriptionAddress,
}

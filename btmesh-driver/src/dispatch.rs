use crate::storage::provisioned::subscriptions::Subscriptions;
use crate::{DriverError, ProvisionedStack};
use btmesh_common::address::UnicastAddress;
use btmesh_common::{ModelIdentifier, Seq};
use btmesh_device::access_counted::AccessCounted;
use btmesh_device::{
    Control, InboundBody, InboundChannelSender, InboundMessage, InboundPayload, PublicationCadence,
};
use btmesh_models::foundation::configuration::ConfigurationServer;
use btmesh_models::Model;
use btmesh_pdu::provisioned::access::AccessMessage;
use core::cmp::Ordering;
use heapless::Vec;
use uluru::LRUCache;

#[derive(PartialEq)]
struct CacheEntry {
    seq: Seq,
    src: UnicastAddress,
    iv_index: u16,
}

pub struct Dispatcher {
    foundation_sender: InboundChannelSender,
    device_sender: InboundChannelSender,
    lru: LRUCache<CacheEntry, 32>,
}

impl Dispatcher {
    pub fn new(
        foundation_sender: InboundChannelSender,
        device_sender: InboundChannelSender,
    ) -> Self {
        Self {
            foundation_sender,
            device_sender,
            lru: Default::default(),
        }
    }

    pub fn check_if_replay(&mut self, pdu: &AccessMessage<ProvisionedStack>) -> bool {
        if let Some(replay_seq) = pdu.meta().replay_seq() {
            let iv_index = (pdu.meta().iv_index().value() & 0xFFFF) as u16;

            if let Some(entry) = self.lru.find(|e| e.src == pdu.meta().src()) {
                match iv_index.cmp(&entry.iv_index) {
                    Ordering::Less => true,
                    Ordering::Equal => {
                        if replay_seq <= entry.seq {
                            true
                        } else {
                            entry.seq = replay_seq;
                            false
                        }
                    }
                    Ordering::Greater => {
                        entry.iv_index = iv_index;
                        entry.seq = replay_seq;
                        false
                    }
                }
            } else {
                self.lru.insert(CacheEntry {
                    seq: replay_seq,
                    src: pdu.meta().src(),
                    iv_index,
                });
                false
            }
        } else {
            false
        }
    }

    pub async fn dispatch(
        &mut self,
        message: &AccessMessage<ProvisionedStack>,
        subscriptions: &Subscriptions,
    ) -> Result<(), DriverError> {
        // TODO figure out my logic issues
        if self.check_if_replay(message) {
            return Ok(());
        }

        let opcode = message.opcode();
        let parameters = message.parameters();
        let local_element_index = message.meta().local_element_index();

        let meta = message.meta().into();

        if let Some(local_element_index) = local_element_index {
            // unicast to an element
            unsafe {
                PAYLOAD.set(InboundPayload {
                    element_index: local_element_index as usize,
                    model_identifier: None,
                    body: InboundBody::Message(InboundMessage {
                        opcode,
                        parameters: Vec::from_slice(parameters)?,
                        meta,
                    }),
                });
            }
            if local_element_index == 0 {
                self.foundation_sender.send(unsafe { PAYLOAD.get() }).await;
            }
            self.device_sender.send(unsafe { PAYLOAD.get() }).await;

            unsafe {
                PAYLOAD.wait().await;
            }
        } else {
            // not unicast, check subscriptions.
            for subscription in subscriptions.subscriptions_for(meta.dst())? {
                unsafe {
                    PAYLOAD.set(InboundPayload {
                        element_index: subscription.element_index as usize,
                        model_identifier: Some(subscription.model_identifier),
                        body: InboundBody::Message(InboundMessage {
                            opcode,
                            parameters: Vec::from_slice(parameters)?,
                            meta,
                        }),
                    });
                }

                // only dispatch to foundation if actually foundational subscription.
                if subscription.element_index == 0
                    && subscription.model_identifier == ConfigurationServer::IDENTIFIER
                {
                    self.foundation_sender.send(unsafe { PAYLOAD.get() }).await;
                }

                self.device_sender.send(unsafe { PAYLOAD.get() }).await;

                unsafe {
                    PAYLOAD.wait().await;
                }
            }
        }

        Ok(())
    }

    pub async fn dispatch_publish(
        &self,
        element_index: u8,
        model_identifier: ModelIdentifier,
        cadence: PublicationCadence,
    ) {
        unsafe {
            PAYLOAD.set(InboundPayload {
                element_index: element_index as usize,
                model_identifier: Some(model_identifier),
                body: InboundBody::Control(Control::PublicationCadence(cadence)),
            });
        }

        if element_index == 0 && model_identifier == ConfigurationServer::IDENTIFIER {
            self.foundation_sender.send(unsafe { PAYLOAD.get() }).await;
        }

        self.device_sender.send(unsafe { PAYLOAD.get() }).await;

        unsafe {
            PAYLOAD.wait().await;
        }
    }
}

static mut PAYLOAD: AccessCounted<InboundPayload> = AccessCounted::new();

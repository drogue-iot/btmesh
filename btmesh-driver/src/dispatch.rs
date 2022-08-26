use crate::{DriverError, ProvisionedStack};
use btmesh_common::address::UnicastAddress;
use btmesh_common::Seq;
use btmesh_device::access_counted::AccessCounted;
use btmesh_device::{InboundChannelSender, InboundPayload};
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
    ) -> Result<(), DriverError> {
        info!("dispatch {}", message);

        // TODO figure out my logic issues
        if self.check_if_replay(message) {
            info!("avoiding replay");
            return Ok(());
        }

        let opcode = message.opcode();
        let parameters = message.parameters();
        let local_element_index = message.meta().local_element_index();

        let meta = message.meta().into();

        unsafe {
            PAYLOAD.set(InboundPayload {
                element_index: local_element_index.map(|index| index as usize),
                opcode,
                parameters: Vec::from_slice(parameters)?,
                meta,
            });
        }

        if let Some(local_element_index) = local_element_index {
            info!("dispatch to {}", local_element_index);
            if local_element_index == 0 {
                self.foundation_sender
                    .send(unsafe { PAYLOAD.get() })
                    //.send((Some(0usize), opcode, Vec::from_slice(parameters)?, meta))
                    .await;
            }
            self.device_sender
                .send(unsafe { PAYLOAD.get() })
                //.send(( Some(local_element_index as usize), opcode, Vec::from_slice(parameters)?, meta,
                .await;
        } else {
            self.foundation_sender
                .send(unsafe { PAYLOAD.get() })
                //.send((None, opcode, Vec::from_slice(parameters)?, meta))
                .await;
            self.device_sender
                .send(unsafe { PAYLOAD.get() })
                //.send((None, opcode, Vec::from_slice(parameters)?, meta))
                .await;
        }

        unsafe {
            PAYLOAD.wait().await;
        }

        info!("dispatch complete");

        Ok(())
    }
}

static mut PAYLOAD: AccessCounted<InboundPayload> = AccessCounted::new();

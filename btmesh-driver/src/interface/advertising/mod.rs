use crate::interface::advertising::segmentation::outbound::{
    OutboundSegments, OutboundSegmentsIter,
};
use crate::interface::advertising::segmentation::Segmentation;
use crate::{DeviceState, Watchdog};
use btmesh_bearer::beacon::Beacon;
use btmesh_bearer::PB_ADV_MTU;
use btmesh_bearer::{AdvertisingBearer, BearerError};
use btmesh_common::Uuid;
use btmesh_pdu::provisioned::network::NetworkPDU;
use btmesh_pdu::provisioning::advertising::AdvertisingPDU;
use btmesh_pdu::provisioning::generic::{
    GenericProvisioningPDU, ProvisioningBearerControl, Reason,
};
use btmesh_pdu::provisioning::ProvisioningPDU;
use btmesh_pdu::{MESH_BEACON, MESH_MESSAGE, PB_ADV, PDU};
use core::cell::Cell;
use core::cell::RefCell;
use core::iter::Iterator;
use embassy_time::{Duration, Instant};
use heapless::Vec;

mod segmentation;

pub struct AdvertisingBearerNetworkInterface<B: AdvertisingBearer> {
    bearer: B,
    segmentation: Segmentation,
    link_id: Cell<Option<u32>>,
    inbound_transaction_number: Cell<Option<u8>>,
    acked_inbound_transaction_number: Cell<Option<u8>>,
    outbound_pdu: RefCell<Option<OutboundPDU>>,
    outbound_transaction_number: Cell<u8>,
}

impl<B: AdvertisingBearer> AdvertisingBearerNetworkInterface<B> {
    pub fn new(bearer: B) -> Self {
        Self {
            bearer,
            segmentation: Default::default(),
            link_id: Cell::new(None),
            inbound_transaction_number: Cell::new(None),
            acked_inbound_transaction_number: Cell::new(None),
            outbound_pdu: RefCell::new(None),
            outbound_transaction_number: Cell::new(0x80),
        }
    }

    pub fn reset(&self) {
        self.link_id.take();
        self.inbound_transaction_number.take();
        self.acked_inbound_transaction_number.take();
        self.outbound_pdu.borrow_mut().take();
        self.outbound_transaction_number.replace(0x80);
    }

    pub async fn beacon(&self, beacon: Beacon) -> Result<(), BearerError> {
        match beacon {
            Beacon::Unprovisioned(uuid) => {
                let mut adv_data: Vec<u8, PB_ADV_MTU> = Vec::new();
                adv_data.extend_from_slice(&[20, MESH_BEACON, 0x00])?;
                adv_data.extend_from_slice(&uuid)?;
                adv_data.extend_from_slice(&[0xa0, 0x40])?;
                self.bearer.transmit(&adv_data).await?;
            }
            Beacon::Provisioned(_network_id) => {
                // not applicable to this role
            }
            Beacon::Secure => {
                // nothing yet.
            }
        }
        Ok(())
    }

    pub async fn transmit(&self, pdu: &PDU) -> Result<(), BearerError> {
        match pdu {
            PDU::Provisioning(pdu) => self.transmit_provisioning_pdu(pdu).await,
            PDU::Network(pdu) => self.transmit_network_pdu(pdu).await,
        }
    }

    #[allow(clippy::await_holding_refcell_ref)]
    async fn transmit_provisioning_pdu(&self, pdu: &ProvisioningPDU) -> Result<(), BearerError> {
        let segments = self.segmentation.process_outbound(pdu)?;

        let transaction_number = self.outbound_transaction_number.get();
        self.outbound_transaction_number
            .replace(transaction_number + 1);

        self.outbound_pdu.replace(Some(OutboundPDU {
            link_id: self.link_id.get().ok_or(BearerError::InvalidLink)?,
            transaction_number,
            segments,
        }));

        if let Some(pdu) = &*self.outbound_pdu.borrow() {
            for pdu in pdu.iter() {
                self.transmit_advertising_pdu(&pdu).await?;
            }
        }
        Ok(())
    }

    async fn transmit_network_pdu(&self, pdu: &NetworkPDU) -> Result<(), BearerError> {
        let mut bytes = Vec::<u8, 64>::new();
        bytes.push(0x00)?;
        bytes.push(MESH_MESSAGE)?;
        pdu.emit(&mut bytes)?;
        bytes[0] = bytes.len() as u8 - 1;
        self.bearer.transmit(&bytes).await?;
        Ok(())
    }

    pub async fn receive(
        &self,
        state: &DeviceState,
        watchdog: &Watchdog,
    ) -> Result<PDU, BearerError> {
        loop {
            let data = self.bearer.receive().await?;
            if data.len() >= 2 {
                match (state, data[1]) {
                    (
                        DeviceState::Unprovisioned {
                            uuid,
                            in_progress: _,
                        },
                        PB_ADV,
                    ) => {
                        if let Some(pdu) = self.receive_pb_adv(&data, uuid, watchdog).await? {
                            return Ok(PDU::Provisioning(pdu));
                        }
                    }
                    (DeviceState::Provisioned, MESH_MESSAGE) => {
                        self.link_id.take();
                        self.inbound_transaction_number.take();
                        if let Ok(pdu) = NetworkPDU::parse(&data[2..]) {
                            return Ok(PDU::Network(pdu));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub async fn close_link(&self, reason: Reason) -> Result<(), BearerError> {
        if let Some(link_id) = self.link_id.get() {
            let pdu = ProvisioningBearerControl::LinkClose(reason);
            let pdu = AdvertisingPDU {
                link_id,
                transaction_number: 0,
                pdu: pdu.into(),
            };
            self.transmit_advertising_pdu(&pdu).await?;
        }
        Ok(())
    }

    async fn receive_pb_adv(
        &self,
        data: &Vec<u8, PB_ADV_MTU>,
        device_uuid: &Uuid,
        watchdog: &Watchdog,
    ) -> Result<Option<ProvisioningPDU>, BearerError> {
        if let Ok(pdu) = AdvertisingPDU::parse(data) {
            match &pdu.pdu {
                GenericProvisioningPDU::ProvisioningBearerControl(pbc) => {
                    match pbc {
                        ProvisioningBearerControl::LinkOpen(uuid) => {
                            if *uuid == *device_uuid {
                                if self.link_id.get().is_none() {
                                    watchdog.link_opening_timeout(
                                        Instant::now() + Duration::from_secs(60),
                                    );
                                    self.inbound_transaction_number
                                        .replace(Some(pdu.transaction_number));
                                    self.link_id.replace(Some(pdu.link_id));

                                    self.transmit_advertising_pdu(&AdvertisingPDU {
                                        link_id: pdu.link_id,
                                        transaction_number: 0,
                                        pdu: GenericProvisioningPDU::ProvisioningBearerControl(
                                            ProvisioningBearerControl::LinkAck,
                                        ),
                                    })
                                    .await?;
                                    Ok(None)
                                } else if let Some(link_id) = self.link_id.get() {
                                    if link_id == pdu.link_id {
                                        // just keep LinkAck'ing it.
                                        self.transmit_advertising_pdu(&AdvertisingPDU {
                                            link_id: pdu.link_id,
                                            transaction_number: 0,
                                            pdu: GenericProvisioningPDU::ProvisioningBearerControl(
                                                ProvisioningBearerControl::LinkAck,
                                            ),
                                        })
                                        .await?;
                                        Ok(None)
                                    } else {
                                        Err(BearerError::InvalidLink)
                                    }
                                } else {
                                    Err(BearerError::InvalidLink)
                                }
                            } else {
                                Ok(None)
                            }
                        }
                        ProvisioningBearerControl::LinkAck => {
                            /* not applicable for this role */
                            Ok(None)
                        }
                        ProvisioningBearerControl::LinkClose(_reason) => {
                            watchdog.clear_link_open_timeout();
                            self.link_id.take();
                            self.inbound_transaction_number.take();
                            //Ok(Some(BearerMessage::Close(*reason)))
                            Ok(None)
                        }
                    }
                }
                GenericProvisioningPDU::TransactionStart(_)
                | GenericProvisioningPDU::TransactionContinuation(_) => {
                    watchdog.clear_link_open_timeout();
                    if self.should_process_transaction(pdu.transaction_number) {
                        let result = self.segmentation.process_inbound(&pdu.pdu);
                        if let Ok(Some(result)) = result {
                            self.ack_transaction().await?;
                            Ok(Some(result))
                        } else {
                            Ok(None)
                        }
                    } else {
                        self.try_ack_transaction_again(pdu.transaction_number)
                            .await?;
                        self.retransmit().await?;
                        Ok(None)
                    }
                }
                GenericProvisioningPDU::TransactionAck => {
                    watchdog.clear_link_open_timeout();
                    let mut borrowed_pdu = self.outbound_pdu.borrow_mut();
                    if let Some(outbound) = &*borrowed_pdu {
                        if outbound.transaction_number == pdu.transaction_number {
                            // They heard us, we can stop retransmitting.
                            borrowed_pdu.take();
                        }
                    }
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    fn should_process_transaction(&self, transaction_number: u8) -> bool {
        match (
            self.inbound_transaction_number.get(),
            self.acked_inbound_transaction_number.get(),
        ) {
            (Some(inbound), _) if inbound == transaction_number => {
                // This transaction is still being collected
                true
            }
            (None, Some(acked)) if acked < transaction_number => {
                // No current transaction, let's go.
                self.inbound_transaction_number
                    .replace(Some(transaction_number));
                true
            }
            _ => {
                // Either current transaction is different or it's already
                // been acked.
                false
            }
        }
    }

    async fn try_ack_transaction_again(&self, transaction_number: u8) -> Result<(), BearerError> {
        if let Some(acked) = self.acked_inbound_transaction_number.get() {
            if acked >= transaction_number {
                self.transmit_advertising_pdu(&AdvertisingPDU {
                    link_id: self.link_id.get().ok_or(BearerError::InvalidLink)?,
                    transaction_number,
                    pdu: GenericProvisioningPDU::TransactionAck,
                })
                .await?;
            }
        }
        Ok(())
    }

    async fn ack_transaction(&self) -> Result<bool, BearerError> {
        match (
            self.inbound_transaction_number.get(),
            self.acked_inbound_transaction_number.get(),
        ) {
            // TODO dry up this repetition
            (Some(current), Some(last_ack)) if current > last_ack => {
                self.transmit_advertising_pdu(&AdvertisingPDU {
                    link_id: self.link_id.get().ok_or(BearerError::InvalidLink)?,
                    transaction_number: current,
                    pdu: GenericProvisioningPDU::TransactionAck,
                })
                .await?;
                self.acked_inbound_transaction_number.replace(Some(current));
                self.inbound_transaction_number.take();
                Ok(true)
            }
            (Some(current), None) => {
                self.transmit_advertising_pdu(&AdvertisingPDU {
                    link_id: self.link_id.get().ok_or(BearerError::InvalidLink)?,
                    transaction_number: current,
                    pdu: GenericProvisioningPDU::TransactionAck,
                })
                .await?;
                self.acked_inbound_transaction_number.replace(Some(current));
                self.inbound_transaction_number.take();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn retransmit(&self) -> Result<(), BearerError> {
        if let Some(outbound) = &*self.outbound_pdu.borrow() {
            for pdu in outbound.iter() {
                self.transmit_advertising_pdu(&pdu).await?
            }
        }
        Ok(())
    }

    async fn transmit_advertising_pdu(&self, pdu: &AdvertisingPDU) -> Result<(), BearerError> {
        let mut bytes = Vec::new();
        pdu.emit(&mut bytes)
            .map_err(|_| BearerError::InsufficientResources)?;
        self.bearer.transmit(&bytes).await
    }
}

pub struct OutboundPDU {
    link_id: u32,
    transaction_number: u8,
    segments: OutboundSegments,
}

impl OutboundPDU {
    pub fn iter(&self) -> OutboundPDUIter {
        OutboundPDUIter {
            link_id: self.link_id,
            transaction_number: self.transaction_number,
            inner: self.segments.iter(),
        }
    }
}

pub struct OutboundPDUIter<'i> {
    link_id: u32,
    transaction_number: u8,
    inner: OutboundSegmentsIter<'i>,
}

impl<'i> OutboundPDUIter<'i> {
    fn new(inner: OutboundSegmentsIter<'i>, link_id: u32, transaction_number: u8) -> Self {
        Self {
            link_id,
            transaction_number,
            inner,
        }
    }
}

impl<'i> Iterator for OutboundPDUIter<'i> {
    type Item = AdvertisingPDU;

    fn next(&mut self) -> Option<Self::Item> {
        let inner = self.inner.next();
        match inner {
            None => None,
            Some(pdu) => Some(AdvertisingPDU {
                link_id: self.link_id,
                transaction_number: self.transaction_number,
                pdu,
            }),
        }
    }
}

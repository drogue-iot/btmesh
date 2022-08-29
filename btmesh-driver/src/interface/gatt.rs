use crate::interface::NetworkError;
use btmesh_bearer::beacon::Beacon;
use btmesh_bearer::{BearerError, GattBearer};
use btmesh_pdu::provisioned::network::NetworkPDU;
use btmesh_pdu::provisioned::proxy::{MessageType, ProxyPDU, SAR};
use btmesh_pdu::provisioning::ProvisioningPDU;
use btmesh_pdu::PDU;
use heapless::Vec;

pub struct GattBearerNetworkInterface<B: GattBearer<MTU>, const MTU: usize> {
    bearer: B,
}

impl<B: GattBearer<MTU>, const MTU: usize> GattBearerNetworkInterface<B, MTU> {
    pub fn new(bearer: B) -> Self {
        Self { bearer }
    }

    pub async fn run(&self) -> Result<(), NetworkError> {
        self.bearer.run().await?;
        Ok(())
    }

    pub fn reset(&self) {
        self.bearer.reset();
    }

    pub async fn receive(&self) -> Result<PDU, BearerError> {
        loop {
            let data = self.bearer.receive().await?;
            let proxy_pdu = ProxyPDU::parse(&data)?;
            if let SAR::Complete = proxy_pdu.sar {
                match proxy_pdu.message_type {
                    MessageType::NetworkPDU => {
                        let pdu = NetworkPDU::parse(&proxy_pdu.data)?;
                        return Ok(PDU::Network(pdu));
                    }
                    MessageType::MeshBeacon => {}
                    MessageType::ProxyConfiguration => {}
                    MessageType::ProvisioningPDU => {
                        let pdu = ProvisioningPDU::parse(&proxy_pdu.data)?;
                        return Ok(PDU::Provisioning(pdu));
                    }
                }
            }
        }
    }

    pub async fn transmit(&self, pdu: &PDU) -> Result<(), BearerError> {
        match pdu {
            PDU::Provisioning(pdu) => {
                let mut all_proxy_data = Vec::<u8, 384>::new();
                pdu.emit(&mut all_proxy_data)?;
                let mut data = Vec::new();
                data.extend_from_slice(&all_proxy_data)?;
                let proxy_pdu = ProxyPDU {
                    sar: SAR::Complete,
                    message_type: MessageType::ProvisioningPDU,
                    data,
                };

                self.transmit_proxy_pdu(&proxy_pdu).await
            }
            PDU::Network(pdu) => {
                let mut all_proxy_data = Vec::<u8, 384>::new();
                pdu.emit(&mut all_proxy_data)?;
                let mut data = Vec::new();
                data.extend_from_slice(&all_proxy_data)?;
                let proxy_pdu = ProxyPDU {
                    sar: SAR::Complete,
                    message_type: MessageType::NetworkPDU,
                    data,
                };

                self.transmit_proxy_pdu(&proxy_pdu).await
            }
        }
    }

    async fn transmit_proxy_pdu(&self, pdu: &ProxyPDU) -> Result<(), BearerError> {
        let mut bytes = Vec::new();
        pdu.emit(&mut bytes)?;
        self.bearer.transmit(&bytes).await
    }

    pub async fn beacon(&self, beacon: Beacon) -> Result<(), BearerError> {
        match beacon {
            Beacon::Unprovisioned(uuid) => {
                let mut adv_data = Vec::new();

                #[rustfmt::skip]
                    adv_data
                        .extend_from_slice(&[
                            0x02, 0x01, 0x06,
                            0x03, 0x03, 0x27, 0x18,
                            0x15, 0x16, 0x27, 0x18
                        ]).unwrap();

                adv_data.extend_from_slice(&uuid)?;

                // TODO fix OOB data values
                adv_data.extend_from_slice(&[0x00, 0x00])?;

                self.bearer.advertise(&adv_data).await?;
            }
            Beacon::Provisioned(network_id) => {
                let mut adv_data = Vec::new();

                #[rustfmt::skip]
                adv_data.extend_from_slice(&[
                    0x02, 0x01, 0x06,
                    0x03, 0x03, 0x28, 0x18,
                    0x0C, 0x16, 0x28, 0x18
                ]).unwrap();

                adv_data.push(0x00)?; // network id
                adv_data.extend_from_slice(&network_id)?;
                self.bearer.advertise(&adv_data).await?;
            }
            Beacon::Secure => {
                // nothing yet
            }
        }

        Ok(())
    }
}

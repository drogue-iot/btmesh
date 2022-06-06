use crate::lower::LowerPDU;
use btmesh_common::{
    address::{Address, UnicastAddress},
    InsufficientBuffer, ParseError,
};
use crate::System;
use heapless::Vec;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum NetMic {
    Access([u8; 4]),
    Control([u8; 8]),
}

/// On-the-wire network PDU as transmitted over a bearer.
#[derive(Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NetworkPDU {
    ivi: u8,
    /* 1 bit */
    nid: u8,
    /* 7 bits */
    obfuscated: [u8; 6],
    encrypted_and_mic: Vec<u8, 28>,
}

impl NetworkPDU {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        let ivi_nid = data[0];
        let ivi = (ivi_nid & 0b10000000) >> 7;
        let nid = ivi_nid & 0b01111111;
        let obfuscated = [data[1], data[2], data[3], data[4], data[5], data[6]];

        let encrypted_and_mic = Vec::from_slice(&data[7..])?;

        Ok(Self {
            ivi,
            nid,
            obfuscated,
            encrypted_and_mic,
        })
    }

    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        let ivi_nid = ((self.ivi & 0b0000001) << 7) | (self.nid & 0b01111111);
        xmit.push(ivi_nid).map_err(|_| InsufficientBuffer)?;
        xmit.extend_from_slice(&self.obfuscated)?;
        xmit.extend_from_slice(&self.encrypted_and_mic)?;
        Ok(())
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CleartextNetworkPDU<S: System> {
    network_key: S::NetworkKeyHandle,
    ivi: u8,
    /* 1 bit */
    nid: u8,
    /* 7 bits */
    ctl: bool, /* 1 bit */
    ttl: u8,
    /* 7 bits */
    seq: u32,
    /* 24 bits */
    src: UnicastAddress,
    dst: Address,
    transport_pdu: Vec<u8, 16>,
}

impl<S: System> CleartextNetworkPDU<S> {}

use crate::provisioned::System;
use crate::PDU;
use btmesh_common::crypto::network::Nid;
use btmesh_common::{
    address::{Address, UnicastAddress},
    Ctl, InsufficientBuffer, Ivi, ParseError, Seq, Ttl,
};
use heapless::Vec;

/// On-the-wire network PDU as transmitted over a bearer.
#[derive(Clone)]
pub struct NetworkPDU {
    ivi: Ivi,
    /* 1 bit */
    nid: Nid,
    /* 7 bits */
    obfuscated: [u8; 6],
    encrypted_and_mic: Vec<u8, 28>,
}

#[cfg(feature = "defmt")]
impl ::defmt::Format for NetworkPDU {
    fn format(&self, fmt: ::defmt::Formatter) {
        ::defmt::write!(
            fmt,
            "NetworkPDU {{ ivi: {}, nid: {}, obfuscated: {:x}, encrypted_and_mic: {:x} }}",
            self.ivi,
            self.nid,
            self.obfuscated,
            self.encrypted_and_mic
        )
    }
}

impl From<NetworkPDU> for PDU {
    fn from(inner: NetworkPDU) -> Self {
        PDU::Network(inner)
    }
}

impl NetworkPDU {
    pub fn encrypted_and_mic(&self) -> &[u8] {
        &self.encrypted_and_mic
    }

    pub fn obfuscated(&self) -> &[u8; 6] {
        &self.obfuscated
    }

    pub fn ivi(&self) -> Ivi {
        self.ivi
    }

    pub fn nid(&self) -> Nid {
        self.nid
    }

    pub fn new(
        ivi: Ivi,
        nid: Nid,
        obfuscated: [u8; 6],
        encrypted_and_mic: &[u8],
    ) -> Result<Self, InsufficientBuffer> {
        Ok(Self {
            ivi,
            nid,
            obfuscated,
            encrypted_and_mic: Vec::from_slice(encrypted_and_mic)?,
        })
    }

    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        let ivi_nid = data[0];
        let ivi = (ivi_nid & 0b10000000) >> 7;
        let nid = ivi_nid & 0b01111111;
        let obfuscated = [data[1], data[2], data[3], data[4], data[5], data[6]];

        let encrypted_and_mic = Vec::from_slice(&data[7..])?;

        Ok(Self {
            ivi: Ivi::parse(ivi)?,
            nid: Nid::parse(nid)?,
            obfuscated,
            encrypted_and_mic,
        })
    }

    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) -> Result<(), InsufficientBuffer> {
        let ivi_nid = ((Into::<u8>::into(self.ivi) & 0b0000001) << 7)
            | (Into::<u8>::into(self.nid) & 0b01111111);
        xmit.push(ivi_nid).map_err(|_| InsufficientBuffer)?;
        xmit.extend_from_slice(&self.obfuscated)?;
        xmit.extend_from_slice(&self.encrypted_and_mic)?;
        Ok(())
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[allow(dead_code)]
pub struct CleartextNetworkPDU<S: System = ()> {
    //network_key: S::NetworkKeyHandle,
    ivi: Ivi,
    nid: Nid,
    /* 7 bits */
    ctl: Ctl,
    /* 1 bit */
    ttl: Ttl,
    /* 7 bits */
    seq: Seq,
    /* 24 bits */
    src: UnicastAddress,
    dst: Address,
    transport_pdu: Vec<u8, 16>,
    meta: S::NetworkMetadata,
}

impl<S: System> CleartextNetworkPDU<S> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ivi: Ivi,
        nid: Nid,
        ctl: Ctl,
        ttl: Ttl,
        seq: Seq,
        src: UnicastAddress,
        dst: Address,
        transport_pdu: &[u8],
        meta: S::NetworkMetadata,
    ) -> Result<Self, InsufficientBuffer> {
        Ok(Self {
            ivi,
            nid,
            ctl,
            ttl,
            seq,
            src,
            dst,
            transport_pdu: Vec::from_slice(transport_pdu)?,
            meta,
        })
    }

    pub fn relay(&self) -> Result<Option<Self>, InsufficientBuffer> {
        if self.ttl.value() == 1 {
            Ok(None)
        } else {
            Ok(Some(Self {
                ivi: self.ivi,
                nid: self.nid,
                ctl: self.ctl,
                ttl: self.ttl.decr(),
                seq: self.seq,
                src: self.src,
                dst: self.dst,
                transport_pdu: Vec::from_slice(&self.transport_pdu)?,
                meta: self.meta.clone(),
            }))
        }
    }

    pub fn ivi(&self) -> Ivi {
        self.ivi
    }

    pub fn nid(&self) -> Nid {
        self.nid
    }

    pub fn ttl(&self) -> Ttl {
        self.ttl
    }

    pub fn ctl(&self) -> Ctl {
        self.ctl
    }

    pub fn seq(&self) -> Seq {
        self.seq
    }

    pub fn src(&self) -> UnicastAddress {
        self.src
    }

    pub fn dst(&self) -> Address {
        self.dst
    }

    pub fn meta(&self) -> &S::NetworkMetadata {
        &self.meta
    }

    pub fn meta_mut(&mut self) -> &mut S::NetworkMetadata {
        &mut self.meta
    }

    pub fn transport_pdu(&self) -> &[u8] {
        &self.transport_pdu
    }
}

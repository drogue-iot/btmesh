use crate::secrets::{NetworkKey, NetworkKeyIter};
use crate::{Driver, DriverError, NetworkKeyHandle, NetworkMetadata, ReplayProtection};
use btmesh_common::address::{Address, UnicastAddress};
use btmesh_common::crypto::nonce::NetworkNonce;
use btmesh_common::{crypto, Ctl, Nid, Seq, Ttl};
use btmesh_pdu::network::{CleartextNetworkPDU, NetworkPDU};
use core::slice::Iter;

pub mod replay_protection;

pub struct NetworkDriver {
    replay_protection: ReplayProtection,
}

impl Driver {
    fn network_keys_by_nid(&self, nid: Nid) -> NetworkKeyIter<'_, Iter<'_, Option<NetworkKey>>> {
        self.secrets.network_keys_by_nid(nid)
    }

    fn privacy_key(&self, network_key: NetworkKeyHandle) -> Result<[u8; 16], DriverError> {
        self.secrets.privacy_key(network_key)
    }

    fn encryption_key(&self, network_key: NetworkKeyHandle) -> Result<[u8; 16], DriverError> {
        self.secrets.encryption_key(network_key)
    }

    pub fn validate_cleartext_network_pdu(&mut self, pdu: &mut CleartextNetworkPDU<Self>) {
        self.network.replay_protection.check(pdu);
    }

    pub fn try_decrypt_network_pdu(
        &self,
        pdu: &NetworkPDU,
        iv_index: u32,
    ) -> Result<CleartextNetworkPDU<Self>, DriverError> {
        for network_key in self.network_keys_by_nid(pdu.nid()) {
            if let Ok(result) = self.try_decrypt_network_pdu_with_key(pdu, iv_index, network_key) {
                return Ok(result);
            }
        }

        Err(DriverError::CryptoError)
    }

    pub fn try_decrypt_network_pdu_with_key(
        &self,
        pdu: &NetworkPDU,
        iv_index: u32,
        network_key: NetworkKeyHandle,
    ) -> Result<CleartextNetworkPDU<Driver>, DriverError> {
        let mut encrypted_and_mic = pdu.encrypted_and_mic().clone();
        let privacy_plaintext = crypto::privacy_plaintext(iv_index, &encrypted_and_mic);

        let pecb = crypto::e(&self.privacy_key(network_key)?, privacy_plaintext)
            .map_err(|_| DriverError::InvalidKeyLength)?;

        let unobfuscated = crypto::pecb_xor(pecb, *pdu.obfuscated());
        let ctl = Ctl::parse(unobfuscated[0] & 0b10000000)?;

        let seq = u32::from_be_bytes([0, unobfuscated[1], unobfuscated[2], unobfuscated[3]]);

        let nonce = NetworkNonce::new(
            unobfuscated[0],
            seq,
            [unobfuscated[4], unobfuscated[5]],
            iv_index,
        );

        let encrypted_len = encrypted_and_mic.len();

        let (payload, mic) = encrypted_and_mic.split_at_mut(encrypted_len - ctl.netmic_size());

        if let Ok(_) = crypto::aes_ccm_decrypt_detached(
            &self.encryption_key(network_key)?,
            &nonce.into_bytes(),
            payload,
            mic,
            None,
        ) {
            let ttl = Ttl::parse(unobfuscated[0] & 0b01111111)?;
            let seq = Seq::parse(u32::from_be_bytes([
                0,
                unobfuscated[1],
                unobfuscated[2],
                unobfuscated[3],
            ]))?;

            let src = UnicastAddress::parse([unobfuscated[4], unobfuscated[5]])?;

            let dst = Address::parse([payload[0], payload[1]]);

            let transport_pdu = &payload[2..];

            let meta = NetworkMetadata {
                iv_index,
                ..Default::default()
            };

            Ok(CleartextNetworkPDU::new(
                network_key,
                pdu.ivi(),
                pdu.nid(),
                ctl,
                ttl,
                seq,
                src,
                dst,
                transport_pdu,
                meta,
            )?)
        } else {
            Err(DriverError::CryptoError)
        }
    }
}

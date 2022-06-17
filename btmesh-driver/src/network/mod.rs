use crate::{Driver, DriverError, IvIndexState, NetworkKeyHandle, NetworkMetadata, ReplayProtection};
use btmesh_common::address::{Address, UnicastAddress};
use btmesh_common::crypto::nonce::NetworkNonce;
use btmesh_common::{crypto, Ctl, IvIndex, Nid, Seq, Ttl};
use btmesh_pdu::network::{CleartextNetworkPDU, NetworkPDU};

pub mod replay_protection;

pub(crate) struct DeviceInfo {
    number_of_elements: u8,
    primary_unicast_address: UnicastAddress,
}

impl DeviceInfo {
    fn new(primary_unicast_address: UnicastAddress, number_of_elements: u8) -> Self {
        Self {
            number_of_elements,
            primary_unicast_address,
        }
    }

    fn local_element_index(&self, dst: Address) -> Option<u8> {
        if let Address::Unicast(dst) = dst {
            if dst >= self.primary_unicast_address {
                let diff = dst - self.primary_unicast_address;
                if diff < self.number_of_elements {
                    return Some(diff);
                }
            }
        }
        None
    }
}

pub struct NetworkDriver {
    device_info: DeviceInfo,
    replay_protection: ReplayProtection,
}

impl NetworkDriver {
    pub(crate) fn new(device_info: DeviceInfo) -> Self {
        Self {
            device_info,
            replay_protection: ReplayProtection::default(),
        }
    }

    #[inline]
    fn local_element_index(&self, dst: Address) -> Option<u8> {
        self.device_info.local_element_index(dst)
    }
}

impl Driver {
    fn network_keys_by_nid(&self, nid: Nid) -> impl Iterator<Item = NetworkKeyHandle> + '_ {
        self.secrets.network_keys_by_nid(nid)
    }

    fn privacy_key(&self, network_key: NetworkKeyHandle) -> Result<[u8; 16], DriverError> {
        self.secrets.network_privacy_key(network_key)
    }

    fn encryption_key(&self, network_key: NetworkKeyHandle) -> Result<[u8; 16], DriverError> {
        self.secrets.network_encryption_key(network_key)
    }

    pub fn validate_cleartext_network_pdu(&mut self, pdu: &mut CleartextNetworkPDU<Self>) {
        self.network.replay_protection.check(pdu);
    }

    pub fn try_decrypt_network_pdu(
        &self,
        pdu: &NetworkPDU,
        iv_index: IvIndex,
    ) -> Result<Option<CleartextNetworkPDU<Driver>>, DriverError> {
        for network_key in self.network_keys_by_nid(pdu.nid()) {
            if let Ok(result) = self.try_decrypt_network_pdu_with_key(pdu, iv_index, network_key) {
                return Ok(Some(result));
            }
        }

        Ok(None)
    }

    pub fn try_decrypt_network_pdu_with_key(
        &self,
        pdu: &NetworkPDU,
        iv_index: IvIndex,
        network_key: NetworkKeyHandle,
    ) -> Result<CleartextNetworkPDU<Driver>, DriverError> {

        let mut encrypted_and_mic = pdu.encrypted_and_mic().clone();
        let privacy_plaintext = crypto::privacy_plaintext(iv_index, &encrypted_and_mic);

        let pecb = crypto::e(&self.privacy_key(network_key)?, privacy_plaintext)
            .map_err(|_| DriverError::InvalidKeyLength)?;

        let unobfuscated = crypto::pecb_xor(pecb, *pdu.obfuscated());
        let ctl = Ctl::parse(unobfuscated[0] & 0b10000000)?;

        let seq = Seq::parse(u32::from_be_bytes([
            0,
            unobfuscated[1],
            unobfuscated[2],
            unobfuscated[3],
        ]))?;

        let nonce = NetworkNonce::new(
            unobfuscated[0],
            seq,
            UnicastAddress::parse([unobfuscated[4], unobfuscated[5]])?,
            iv_index,
        );

        let encrypted_len = encrypted_and_mic.len();

        let (payload, mic) = encrypted_and_mic.split_at_mut(encrypted_len - ctl.netmic_size());

        if crypto::aes_ccm_decrypt_detached(
            &self.encryption_key(network_key)?,
            &nonce.into_bytes(),
            payload,
            mic,
            None,
        )
        .is_ok()
        {
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

            let local_element_index = self.network.local_element_index(dst);

            let meta = NetworkMetadata {
                iv_index,
                replay_protected: false,
                should_relay: false,
                local_element_index,
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

#[cfg(test)]
mod test {
    use crate::network::DeviceInfo;
    use btmesh_common::address::{Address, GroupAddress, UnicastAddress};

    #[test]
    fn local_element_index() {
        let device_info = DeviceInfo::new(UnicastAddress::parse([0x00, 0x0A]).unwrap(), 3);

        assert_eq!(
            None,
            device_info.local_element_index(UnicastAddress::parse([0x00, 0x01]).unwrap().into())
        );
        assert_eq!(
            None,
            device_info.local_element_index(UnicastAddress::parse([0x00, 0x02]).unwrap().into())
        );
        assert_eq!(
            None,
            device_info.local_element_index(UnicastAddress::parse([0x00, 0x09]).unwrap().into())
        );

        assert_eq!(
            Some(0),
            device_info.local_element_index(UnicastAddress::parse([0x00, 0x0A]).unwrap().into())
        );
        assert_eq!(
            Some(1),
            device_info.local_element_index(UnicastAddress::parse([0x00, 0x0B]).unwrap().into())
        );
        assert_eq!(
            Some(2),
            device_info.local_element_index(UnicastAddress::parse([0x00, 0x0C]).unwrap().into())
        );

        assert_eq!(
            None,
            device_info.local_element_index(UnicastAddress::parse([0x00, 0x0D]).unwrap().into())
        );
        assert_eq!(
            None,
            device_info.local_element_index(UnicastAddress::parse([0x00, 0x0E]).unwrap().into())
        );

        assert_eq!(None, device_info.local_element_index(Address::Unassigned));
        assert_eq!(
            None,
            device_info.local_element_index(Address::Group(GroupAddress::AllNodes))
        );
    }
}

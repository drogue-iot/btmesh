use crate::provisioned::system::{NetworkKeyHandle, NetworkMetadata};
use crate::provisioned::{DriverError, ProvisionedDriver, ReplayProtection};
use btmesh_common::address::{Address, UnicastAddress};
use btmesh_common::crypto::network::{NetMic, NetworkKey, Nid};
use btmesh_common::crypto::nonce::NetworkNonce;
use btmesh_common::{crypto, Ctl, IvIndex, Seq, Ttl};
use btmesh_pdu::network::{CleartextNetworkPDU, NetworkPDU};
use heapless::Vec;

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

impl ProvisionedDriver {
    fn network_keys_by_nid(&self, nid: Nid) -> impl Iterator<Item = NetworkKeyHandle> + '_ {
        self.secrets.network_keys_by_nid(nid)
    }

    fn network_key(&self, handle: NetworkKeyHandle) -> Result<NetworkKey, DriverError> {
        self.secrets.network_key(handle)
    }

    pub fn validate_cleartext_network_pdu(&mut self, pdu: &mut CleartextNetworkPDU<Self>) {
        self.network.replay_protection.check(pdu);
    }

    pub fn encrypt_network_pdu(
        &self,
        cleartext_pdu: &CleartextNetworkPDU<ProvisionedDriver>,
    ) -> Result<NetworkPDU, DriverError> {
        let ctl_ttl = match cleartext_pdu.ctl() {
            Ctl::Access => 0,
            Ctl::Control => 1,
        } << 7
            | cleartext_pdu.ttl().value();

        let nonce = NetworkNonce::new(
            ctl_ttl,
            cleartext_pdu.seq(),
            cleartext_pdu.src(),
            cleartext_pdu.meta().iv_index(),
        );

        let mut encrypted_and_mic = Vec::<_, 28>::new();
        encrypted_and_mic
            .extend_from_slice(&cleartext_pdu.dst().as_bytes())
            .map_err(|_| DriverError::InsufficientSpace)?;
        encrypted_and_mic
            .extend_from_slice(cleartext_pdu.transport_pdu())
            .map_err(|_| DriverError::InsufficientSpace)?;

        let network_key = self.network_key(cleartext_pdu.meta().network_key_handle())?;

        match cleartext_pdu.ctl() {
            Ctl::Access => {
                let mut mic = NetMic::new_access();

                crypto::network::encrypt_network(
                    &network_key,
                    &nonce,
                    &mut encrypted_and_mic,
                    &mut mic,
                )
                .map_err(|_| DriverError::CryptoError)?;

                encrypted_and_mic
                    .extend_from_slice(mic.as_ref())
                    .map_err(|_| DriverError::InsufficientSpace)?;
            }
            Ctl::Control => {
                let mut mic = NetMic::new_control();

                crypto::network::encrypt_network(
                    &network_key,
                    &nonce,
                    &mut encrypted_and_mic,
                    &mut mic,
                )
                .map_err(|_| DriverError::CryptoError)?;

                encrypted_and_mic
                    .extend_from_slice(mic.as_ref())
                    .map_err(|_| DriverError::InsufficientSpace)?;
            }
        }

        let privacy_plaintext =
            crypto::privacy_plaintext(cleartext_pdu.meta().iv_index(), &encrypted_and_mic);

        //let privacy_key = self.privacy_key(cleartext_pdu.meta().network_key_handle())?;

        let pecb = crypto::e(&network_key.privacy_key(), privacy_plaintext)
            .map_err(|_| DriverError::CryptoError)?;

        let mut unobfuscated = [0; 6];
        unobfuscated[0] = ctl_ttl;

        let seq_bytes = cleartext_pdu.seq().to_be_bytes();
        unobfuscated[1] = seq_bytes[1];
        unobfuscated[2] = seq_bytes[2];
        unobfuscated[3] = seq_bytes[3];

        let src_bytes = cleartext_pdu.src().as_bytes();
        unobfuscated[4] = src_bytes[0];
        unobfuscated[5] = src_bytes[1];
        let obfuscated = crypto::pecb_xor(pecb, unobfuscated);

        Ok(NetworkPDU::new(
            cleartext_pdu.ivi(),
            cleartext_pdu.nid(),
            obfuscated,
            &*encrypted_and_mic,
        )?)
    }

    pub fn try_decrypt_network_pdu(
        &mut self,
        pdu: &NetworkPDU,
        iv_index: IvIndex,
    ) -> Result<Option<CleartextNetworkPDU<ProvisionedDriver>>, DriverError> {
        let mut result = None;
        for network_key in self.network_keys_by_nid(pdu.nid()) {
            if let Ok(pdu) = self.try_decrypt_network_pdu_with_key(pdu, iv_index, network_key) {
                result.replace(pdu);
                break;
            }
        }

        if let Some(result) = &mut result {
            self.validate_cleartext_network_pdu(result);
        }

        Ok(result)
    }

    pub fn try_decrypt_network_pdu_with_key(
        &self,
        pdu: &NetworkPDU,
        iv_index: IvIndex,
        network_key_handle: NetworkKeyHandle,
    ) -> Result<CleartextNetworkPDU<ProvisionedDriver>, DriverError> {
        let network_key = self.network_key(network_key_handle)?;
        let mut encrypted_and_mic = Vec::<_, 28>::from_slice(pdu.encrypted_and_mic())
            .map_err(|_| DriverError::InsufficientSpace)?;
        let privacy_plaintext = crypto::privacy_plaintext(iv_index, &encrypted_and_mic);

        let pecb = crypto::e(&network_key.privacy_key(), privacy_plaintext)
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

        let mic = NetMic::parse(mic)?;

        if crypto::network::try_decrypt_network(&network_key, &nonce, payload, &mic).is_ok() {
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

            let meta = NetworkMetadata::new(iv_index, local_element_index, network_key_handle);

            Ok(CleartextNetworkPDU::new(
                pdu.ivi(),
                network_key.nid(),
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
    use crate::provisioned::network::DeviceInfo;
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

use super::phases::*;
use crate::DriverError;
use btmesh_common::crypto::device::DeviceKey;
use btmesh_pdu::provisioning::{
    Capabilities, ErrorCode, Failed, ProvisioningData, ProvisioningPDU,
};
use rand_core::{CryptoRng, RngCore};

pub enum Provisionee {
    Beaconing(Phase<Beaconing>),
    Invitation(Phase<Invitation>),
    KeyExchange(Phase<KeyExchange>),
    Authentication(Phase<Authentication>),
    DataDistribution(Phase<DataDistribution>),
    Complete(DeviceKey, ProvisioningData),
    Failure,
}

impl Provisionee {
    pub fn new(capabilities: Capabilities) -> Self {
        Self::Beaconing(Phase::<Beaconing>::new(capabilities))
    }

    pub fn in_progress(&self) -> bool {
        !matches!(
            self,
            Self::Beaconing(..) | Self::Complete(..) | Self::Failure
        )
    }

    pub fn next<RNG: RngCore + CryptoRng>(
        self,
        pdu: &ProvisioningPDU,
        rng: &mut RNG,
    ) -> Result<(Self, Option<ProvisioningPDU>), DriverError> {
        match (self, pdu) {
            // INVITE
            (Provisionee::Beaconing(mut device), ProvisioningPDU::Invite(invite)) => {
                let response = device.invite(invite)?;
                Ok((
                    Provisionee::Invitation(device.into()),
                    Some(ProvisioningPDU::Capabilities(response)),
                ))
            }
            // START
            (Provisionee::Invitation(mut device), ProvisioningPDU::Start(start)) => {
                // TODO: spec says to set the "Attention Timer" to 0x00
                device.start(start, rng)?;
                // TODO: actually let the device/app/thingy know what
                // it is so that it can blink/flash/accept input
                Ok((Provisionee::KeyExchange(device.into()), None))
            }
            // PUBLIC KEY
            (Provisionee::KeyExchange(mut device), ProvisioningPDU::PublicKey(peer_key)) => {
                match device.calculate_ecdh_device(peer_key, rng) {
                    Ok(key) => Ok((
                        Provisionee::Authentication(device.into()),
                        Some(ProvisioningPDU::PublicKey(key)),
                    )),
                    Err(DriverError::Parse(_)) => Provisionee::fail(ErrorCode::InvalidFormat),
                    Err(_) => Provisionee::fail(ErrorCode::UnexpectedError),
                }
            }
            // CONFIRMATION
            (Provisionee::Authentication(mut device), ProvisioningPDU::Confirmation(value)) => {
                let response = device.swap_confirmation(value, rng)?;
                Ok((
                    Provisionee::Authentication(device),
                    Some(ProvisioningPDU::Confirmation(response)),
                ))
            }
            // RANDOM
            (Provisionee::Authentication(mut device), ProvisioningPDU::Random(value)) => {
                match device.check_confirmation(value) {
                    Ok(response) => Ok((
                        Provisionee::DataDistribution(device.into()),
                        Some(ProvisioningPDU::Random(response)),
                    )),
                    Err(_) => Provisionee::fail(ErrorCode::ConfirmationFailed),
                }
            }
            // DATA
            (Provisionee::DataDistribution(device), ProvisioningPDU::Data(data)) => {
                let (device_key, decrypted) = device.decrypt(data)?;
                Ok((
                    Provisionee::Complete(device_key, ProvisioningData::parse(&decrypted)?),
                    Some(ProvisioningPDU::Complete),
                ))
            }
            (current, _) => {
                // if it's an invalid PDU, assume it's just a wayward PDU and ignore, don't break.
                Ok((current, None))
            }
        }
    }

    fn fail(error_code: ErrorCode) -> Result<(Provisionee, Option<ProvisioningPDU>), DriverError> {
        Ok((
            Provisionee::Failure,
            Some(ProvisioningPDU::Failed(Failed { error_code })),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use btmesh_pdu::provisioning::{Confirmation, Invite, PublicKey, Random, Start};
    use p256::SecretKey;
    use rand_core::OsRng;

    #[test]
    fn invitation() {
        let size = 69;
        let caps = Capabilities {
            number_of_elements: size,
            ..Default::default()
        };
        let fsm = Provisionee::new(caps);
        assert!(matches!(fsm, Provisionee::Beaconing(_)));
        let pdu = ProvisioningPDU::Invite(Invite {
            attention_duration: 30,
        });
        let (fsm, pdu) = fsm.next(&pdu, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::Invitation(_)));
        match pdu {
            Some(ProvisioningPDU::Capabilities(c)) => assert_eq!(c.number_of_elements, size),
            _ => panic!("wrong pdu returned for invite"),
        }
    }

    #[test]
    fn valid_keyexchange() {
        let fsm = keyexchange();
        let private = SecretKey::random(OsRng);
        let pdu = ProvisioningPDU::PublicKey(PublicKey::try_from(private.public_key()).unwrap());
        let (fsm, pdu) = fsm.next(&pdu, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::Authentication(_)));
        assert!(matches!(pdu, Some(ProvisioningPDU::PublicKey(_))));
    }

    #[test]
    fn invalid_keyexchange() {
        let fsm = keyexchange();
        let (x, y) = ([0; 32], [0; 32]);
        let pdu = ProvisioningPDU::PublicKey(PublicKey { x, y });
        let (fsm, pdu) = fsm.next(&pdu, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::Failure));
        assert!(
            matches!(pdu, Some(ProvisioningPDU::Failed(e)) if matches!(e.error_code, ErrorCode::InvalidFormat))
        );
    }

    #[test]
    fn valid_confirmation() {
        let mut random = [0; 16];
        OsRng.fill_bytes(&mut random);
        let fsm = confirmation(&random);
        let pdu = ProvisioningPDU::Random(Random { random });
        let (fsm, _pdu) = fsm.next(&pdu, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::DataDistribution(_)));
    }

    #[test]
    fn invalid_confirmation() {
        let mut random = [0; 16];
        let fsm = confirmation(&random);
        // Use a different random to break confirmation...
        OsRng.fill_bytes(&mut random);
        let pdu = ProvisioningPDU::Random(Random { random });
        let (fsm, _pdu) = fsm.next(&pdu, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::Failure));
    }

    fn keyexchange() -> Provisionee {
        let fsm = Provisionee::new(Capabilities::default());
        let invite = ProvisioningPDU::Invite(Invite::default());
        let (fsm, _) = fsm.next(&invite, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::Invitation(_)));
        let start = ProvisioningPDU::Start(Start::default());
        let (fsm, pdu) = fsm.next(&start, &mut OsRng).unwrap();
        assert!(matches!(fsm, Provisionee::KeyExchange(_)));
        assert!(matches!(pdu, None));
        fsm
    }

    fn confirmation(random: &[u8]) -> Provisionee {
        let fsm = keyexchange();
        let private = SecretKey::random(OsRng);
        let pdu = ProvisioningPDU::PublicKey(PublicKey::try_from(private.public_key()).unwrap());
        let (fsm, _pdu) = fsm.next(&pdu, &mut OsRng).unwrap();
        let confirmation = match &fsm {
            Provisionee::Authentication(ref auth) => auth.confirm(random).unwrap(),
            _ => panic!("wrong state returned"),
        };
        let pdu = ProvisioningPDU::Confirmation(Confirmation { confirmation });
        let (fsm, pdu) = fsm.next(&pdu, &mut OsRng).unwrap();
        match pdu {
            Some(ProvisioningPDU::Confirmation(c)) => assert_ne!(c.confirmation, confirmation),
            _ => panic!("wrong pdu returned"),
        }
        fsm
    }
}

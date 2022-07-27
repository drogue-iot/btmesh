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
            (Provisionee::Beaconing(mut phase), ProvisioningPDU::Invite(invite)) => {
                let response = phase.invite(invite)?;
                Ok((
                    Provisionee::Invitation(phase.try_into()?),
                    Some(ProvisioningPDU::Capabilities(response)),
                ))
            }
            // START
            (Provisionee::Invitation(mut phase), ProvisioningPDU::Start(start)) => {
                // TODO: spec says to set the "Attention Timer" to 0x00
                phase.start(start, rng)?;
                // TODO: actually let the device/app/thingy know what
                // it is so that it can blink/flash/accept input
                Ok((Provisionee::KeyExchange(phase.try_into()?), None))
            }
            // PUBLIC KEY
            (Provisionee::KeyExchange(mut phase), ProvisioningPDU::PublicKey(peer_key)) => {
                match phase.calculate_ecdh_device(peer_key, rng) {
                    Ok(key) => Ok((
                        Provisionee::Authentication(phase.try_into()?),
                        Some(ProvisioningPDU::PublicKey(key)),
                    )),
                    Err(DriverError::Parse(_)) => Provisionee::fail(ErrorCode::InvalidFormat),
                    Err(_) => Provisionee::fail(ErrorCode::UnexpectedError),
                }
            }
            // CONFIRMATION
            (Provisionee::Authentication(mut phase), ProvisioningPDU::Confirmation(value)) => {
                let response = phase.device_confirmation(value, rng)?;
                Ok((
                    Provisionee::Authentication(phase),
                    Some(ProvisioningPDU::Confirmation(response)),
                ))
            }
            // RANDOM
            (Provisionee::Authentication(mut phase), ProvisioningPDU::Random(value)) => {
                match phase.device_check(value) {
                    Ok(response) => Ok((
                        Provisionee::DataDistribution(phase.try_into()?),
                        Some(ProvisioningPDU::Random(response)),
                    )),
                    Err(_) => Provisionee::fail(ErrorCode::ConfirmationFailed),
                }
            }
            // DATA
            (Provisionee::DataDistribution(phase), ProvisioningPDU::Data(data)) => {
                let (device_key, decrypted) = phase.decrypt(data)?;
                let data = ProvisioningData::parse(&decrypted)?;
                Ok((
                    Provisionee::Complete(device_key, data),
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

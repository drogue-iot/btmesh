use super::auth_value::determine_auth_value;
use super::phases::*;
use super::transcript::Transcript;
use crate::DriverError;
use btmesh_common::crypto::device::DeviceKey;
use btmesh_pdu::provisioning::{
    Capabilities, Confirmation, ErrorCode, Failed, ProvisioningData, ProvisioningPDU, Random,
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
        Self::Beaconing(Phase {
            transcript: Transcript::default(),
            state: Beaconing { capabilities },
        })
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
            (Provisionee::Beaconing(mut device), ProvisioningPDU::Invite(invite)) => {
                let capabilities = device.state.capabilities.clone();
                device.transcript.add_invite(invite)?;
                device.transcript.add_capabilities(&capabilities)?;
                Ok((
                    Provisionee::Invitation(device.into()),
                    Some(ProvisioningPDU::Capabilities(capabilities)),
                ))
            }
            (Provisionee::Invitation(mut device), ProvisioningPDU::Start(start)) => {
                // TODO: spec says to set the "Attention Timer" to 0x00
                device.transcript.add_start(start)?;
                device.state.auth_value = determine_auth_value(rng, start)?;
                // TODO: actually let the device/app/thingy know what
                // it is so that it can blink/flash/accept input
                Ok((Provisionee::KeyExchange(device.into()), None))
            }
            (Provisionee::KeyExchange(mut device), ProvisioningPDU::PublicKey(peer_key)) => {
                match device.calculate_ecdh(peer_key, rng) {
                    Ok(Some(key)) => Ok((
                        Provisionee::Authentication(device.into()),
                        Some(ProvisioningPDU::PublicKey(key)),
                    )),
                    Ok(None) => Ok((Provisionee::Authentication(device.into()), None)),
                    Err(DriverError::Parse(_)) => Provisionee::fail(ErrorCode::InvalidFormat),
                    Err(e) => Err(e),
                }
            }
            (Provisionee::Authentication(mut device), ProvisioningPDU::Confirmation(value)) => {
                device.state.confirmation = Some(value.confirmation);
                let mut random_device = [0; 16];
                rng.fill_bytes(&mut random_device);
                let confirmation = device.confirm(&random_device)?;
                device.state.random_device = Some(random_device);
                Ok((
                    Provisionee::Authentication(device),
                    Some(ProvisioningPDU::Confirmation(Confirmation { confirmation })),
                ))
            }
            (Provisionee::Authentication(mut device), ProvisioningPDU::Random(value)) => {
                let confirmation = device.confirm(&value.random)?;
                match device.state.confirmation {
                    Some(v) if v == confirmation => (),
                    _ => return Provisionee::fail(ErrorCode::ConfirmationFailed),
                }
                device.state.random_provisioner = Some(value.random);
                let device_random = device.state.random_device.ok_or(DriverError::CryptoError)?;
                Ok((
                    Provisionee::DataDistribution(device.into()),
                    Some(ProvisioningPDU::Random(Random {
                        random: device_random,
                    })),
                ))
            }
            (Provisionee::DataDistribution(device), ProvisioningPDU::Data(data)) => {
                let (device_key, decrypted) = device.decrypt(data)?;
                Ok((
                    Provisionee::Complete(device_key, ProvisioningData::parse(&decrypted)?),
                    Some(ProvisioningPDU::Complete),
                ))
            }
            (current_state, _) => {
                // if it's an invalid PDU, assume it's just a wayward PDU and ignore, don't break.
                Ok((current_state, None))
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
    use btmesh_pdu::provisioning::{Invite, PublicKey, Start};
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

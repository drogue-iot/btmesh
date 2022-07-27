use super::phases::*;
use crate::DriverError;
use btmesh_pdu::provisioning::{
    Confirmation, ErrorCode, Failed, Invite, ProvisioningData, ProvisioningPDU,
};
use rand_core::{CryptoRng, RngCore};

pub enum Provisioner {
    Invitation(Phase<Invitation>),
    KeyExchange(Phase<KeyExchange>),
    Authentication(Phase<Authentication>),
    DataDistribution(Phase<DataDistribution>),
    Success,
    Failure,
}

pub enum ResponsePDU {
    Two([ProvisioningPDU; 2]),
    One(ProvisioningPDU),
    None,
}

impl Provisioner {
    pub fn new(invite: &Invite, data: ProvisioningData) -> Result<Self, DriverError> {
        Ok(Self::Invitation(Phase::<Invitation>::new(invite, data)?))
    }

    pub fn next<RNG: RngCore + CryptoRng>(
        self,
        pdu: &ProvisioningPDU,
        rng: &mut RNG,
    ) -> Result<(Self, ResponsePDU), DriverError> {
        match (self, pdu) {
            // CAPABILITIES
            (Provisioner::Invitation(mut phase), ProvisioningPDU::Capabilities(caps)) => {
                // TODO: This is when we know how many elements the
                // device has. How do we let the caller of this state
                // machine know that we need to reserve the
                // data.unicast_address passed to Provisioner::new
                // plus caps.number_of_elements?
                let (start, pk) = phase.capabilities(caps, rng)?;
                Ok((
                    Provisioner::KeyExchange(phase.try_into()?),
                    ResponsePDU::Two([
                        ProvisioningPDU::Start(start),
                        ProvisioningPDU::PublicKey(pk),
                    ]),
                ))
            }
            // PUBLIC KEY
            (Provisioner::KeyExchange(mut phase), ProvisioningPDU::PublicKey(peer_key)) => {
                // TODO: OOB capabilities should determine whether we
                // return a Confirmation here or wait for the device
                // to send us an InputComplete

                // TODO: Deal better with ErrorCode / ParseError / DriverError

                // match phase.calculate_ecdh_provisioner(peer_key, rng) {
                //     Ok(_key) => Ok((Provisioner::Authentication(phase.into()), ResponsePDU::None)),
                //     Err(DriverError::Parse(_)) => Provisioner::fail(ErrorCode::InvalidFormat),
                //     Err(_) => Provisioner::fail(ErrorCode::UnexpectedError),
                // }

                // TODO: this better, i.e. calculate sets random_provisioner and then returns it... wtf?
                let random = phase.calculate_ecdh_provisioner(peer_key, rng)?;
                let phase: Phase<Authentication> = phase.try_into()?;
                let confirmation = phase.confirm(&random)?;
                let pdu = ProvisioningPDU::Confirmation(Confirmation { confirmation });
                Ok((Provisioner::Authentication(phase), ResponsePDU::One(pdu)))
            }
            // CONFIRMATION
            (Provisioner::Authentication(mut phase), ProvisioningPDU::Confirmation(value)) => {
                let response = phase.provisioner_confirmation(value)?;
                Ok((
                    Provisioner::Authentication(phase),
                    ResponsePDU::One(ProvisioningPDU::Random(response)),
                ))
            }
            // RANDOM
            (Provisioner::Authentication(mut phase), ProvisioningPDU::Random(value)) => {
                if phase.provisioner_check(value).is_ok() {
                    let phase: Phase<DataDistribution> = phase.try_into()?;
                    let response = phase.encrypt()?;
                    Ok((
                        Provisioner::DataDistribution(phase),
                        ResponsePDU::One(ProvisioningPDU::Data(response)),
                    ))
                } else {
                    Provisioner::fail(ErrorCode::ConfirmationFailed)
                }
            }
            // COMPLETE
            (Provisioner::DataDistribution(_), ProvisioningPDU::Complete) => {
                Ok((Provisioner::Success, ResponsePDU::None))
            }
            (current, _) => {
                // if it's an invalid PDU, assume it's just a wayward PDU and ignore, don't break.
                Ok((current, ResponsePDU::None))
            }
        }
    }
    fn fail(error_code: ErrorCode) -> Result<(Provisioner, ResponsePDU), DriverError> {
        Ok((
            Provisioner::Failure,
            ResponsePDU::One(ProvisioningPDU::Failed(Failed { error_code })),
        ))
    }
}

#[cfg(test)]
mod tests {
    use core::ops::Deref;

    use crate::stack::unprovisioned::provisionee::Provisionee;

    use super::*;
    use btmesh_common::{address::UnicastAddress, KeyRefreshFlag};
    use btmesh_pdu::provisioning::Capabilities;
    use rand_core::OsRng;

    #[test]
    fn provision_device() {
        let rng = &mut OsRng;

        let data = ProvisioningData {
            unicast_address: UnicastAddress::new(0x00_0A).unwrap(),
            key_refresh_flag: KeyRefreshFlag(true),
            ..Default::default()
        };
        let invite = Invite::default();
        let caps = Capabilities {
            number_of_elements: 1,
            ..Default::default()
        };

        let mut provisioner = Provisioner::new(&invite, data).unwrap();
        let mut device = Provisionee::new(caps);

        let mut pdu = ProvisioningPDU::Invite(invite);
        let mut result: Option<ProvisioningPDU>;
        let mut response: ResponsePDU;

        loop {
            (device, result) = device.next(&pdu, rng).unwrap();
            match result {
                Some(p) => {
                    pdu = p;
                    match provisioner.next(&pdu, rng) {
                        Ok(x) => (provisioner, response) = x,
                        Err(e) => panic!("unexpected: {:?}, PDU: {:?}", e, pdu),
                    }
                    match response {
                        ResponsePDU::Two(pdus) => {
                            // We don't expect the device to respond to the 1st PDU
                            (device, result) = device.next(&pdus[0], rng).unwrap();
                            assert!(matches!(result, None));
                            pdu = pdus[1].clone();
                        }
                        ResponsePDU::One(p) => pdu = p,
                        ResponsePDU::None => assert!(matches!(pdu, ProvisioningPDU::Complete)),
                    }
                }
                None => break,
            }
        }

        match device {
            Provisionee::Complete(key, result) => {
                assert_ne!(&[0; 16], key.deref());
                assert_eq!(data, result);
            }
            _ => panic!("wrong ending state"),
        }
    }
}

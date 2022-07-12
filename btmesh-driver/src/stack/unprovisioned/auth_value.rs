use btmesh_common::ParseError;
use btmesh_pdu::provisioning::{InputOOBAction, OOBAction, OOBSize, OutputOOBAction, Start};
use heapless::Vec;
use rand_core::RngCore;

#[derive(Default)]
pub enum AuthValue {
    #[default]
    None,
    InputEvents(u32),
    OutputEvents(u32),
    InputNumeric(u32),
    OutputNumeric(u32),
    InputAlphanumeric(Vec<u8, 8>),
    OutputAlphanumeric(Vec<u8, 8>),
}

impl AuthValue {
    pub fn get_bytes(&self) -> [u8; 16] {
        let mut bytes = [0; 16];
        match self {
            AuthValue::None => {
                // all zeros
            }
            AuthValue::InputEvents(num)
            | AuthValue::OutputEvents(num)
            | AuthValue::InputNumeric(num)
            | AuthValue::OutputNumeric(num) => {
                let num_bytes = num.to_be_bytes();
                bytes[12] = num_bytes[0];
                bytes[13] = num_bytes[1];
                bytes[14] = num_bytes[2];
                bytes[15] = num_bytes[3];
            }
            AuthValue::InputAlphanumeric(chars) | AuthValue::OutputAlphanumeric(chars) => {
                for (i, byte) in chars.iter().enumerate() {
                    bytes[i] = *byte
                }
            }
        }

        bytes
    }
}

pub fn determine_auth_value<RNG: RngCore>(
    rng: &mut RNG,
    start: &Start,
) -> Result<AuthValue, ParseError> {
    Ok(
        match (&start.authentication_action, &start.authentication_size) {
            (
                OOBAction::Output(OutputOOBAction::Blink)
                | OOBAction::Output(OutputOOBAction::Beep)
                | OOBAction::Output(OutputOOBAction::Vibrate),
                OOBSize::MaximumSize(size),
            ) => {
                let auth_raw = random_physical_oob(rng, *size);
                AuthValue::OutputEvents(auth_raw)
            }
            (
                OOBAction::Input(InputOOBAction::Push) | OOBAction::Input(InputOOBAction::Twist),
                OOBSize::MaximumSize(size),
            ) => {
                let auth_raw = random_physical_oob(rng, *size);
                AuthValue::InputEvents(auth_raw)
            }
            (OOBAction::Output(OutputOOBAction::OutputNumeric), OOBSize::MaximumSize(size)) => {
                let auth_raw = random_numeric(rng, *size);
                AuthValue::OutputNumeric(auth_raw)
            }
            // TODO actually dispatch to device/app/thing's UI for inputs instead of just making up shit.
            (OOBAction::Input(InputOOBAction::InputNumeric), OOBSize::MaximumSize(size)) => {
                let auth_raw = random_numeric(rng, *size);
                AuthValue::InputNumeric(auth_raw)
            }
            (
                OOBAction::Output(OutputOOBAction::OutputAlphanumeric),
                OOBSize::MaximumSize(size),
            ) => {
                let auth_raw = random_alphanumeric(rng, *size)?;
                AuthValue::OutputAlphanumeric(auth_raw)
            }
            (OOBAction::Input(InputOOBAction::InputAlphanumeric), OOBSize::MaximumSize(size)) => {
                let auth_raw = random_alphanumeric(rng, *size)?;
                AuthValue::InputAlphanumeric(auth_raw)
            }
            _ => {
                // zeros!
                AuthValue::None
            }
        },
    )
}

fn random_physical_oob<RNG: RngCore>(rng: &mut RNG, size: u8) -> u32 {
    // "select a random integer between 0 and 10 to the power of the Authentication Size exclusive"
    //
    // ... which could be an absolute metric tonne of beeps/twists/pushes if AuthSize is large-ish.
    let mut max = 1;
    for _ in 0..size {
        max *= 10;
    }

    loop {
        let candidate = rng.next_u32();
        if candidate > 0 && candidate < max {
            return candidate;
        }
    }
}

fn random_numeric<RNG: RngCore>(rng: &mut RNG, size: u8) -> u32 {
    loop {
        let candidate = rng.next_u32();

        match size {
            1 => {
                if candidate < 10 {
                    return candidate;
                }
            }
            2 => {
                if candidate < 100 {
                    return candidate;
                }
            }
            3 => {
                if candidate < 1_000 {
                    return candidate;
                }
            }
            4 => {
                if candidate < 10_000 {
                    return candidate;
                }
            }
            5 => {
                if candidate < 100_000 {
                    return candidate;
                }
            }
            6 => {
                if candidate < 1_000_000 {
                    return candidate;
                }
            }
            7 => {
                if candidate < 10_000_000 {
                    return candidate;
                }
            }
            8 => {
                if candidate < 100_000_000 {
                    return candidate;
                }
            }
            _ => {
                // should never get here, but...
                return 0;
            }
        }
    }
}

fn random_alphanumeric<RNG: RngCore>(rng: &mut RNG, size: u8) -> Result<Vec<u8, 8>, ParseError> {
    let mut random = Vec::new();
    for _ in 0..size {
        loop {
            let candidate = (rng.next_u32() & 0xFF) as u8;
            if (64..=90).contains(&candidate) {
                // Capital ASCII letters A-Z
                random
                    .push(candidate)
                    .map_err(|_| ParseError::InsufficientBuffer)?;
            } else if (48..=57).contains(&candidate) {
                // ASCII numbers 0-9
                random
                    .push(candidate)
                    .map_err(|_| ParseError::InsufficientBuffer)?;
            }
        }
    }
    Ok(random)
}

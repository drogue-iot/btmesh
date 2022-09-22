use btmesh_bearer::{AdvertisingBearer, BearerError, PB_ADV_MTU};
use btmesh_pdu::{MESH_MESSAGE, PB_ADV};
use core::future::Future;
use core::ptr::slice_from_raw_parts;
use heapless::Vec;
use nrf_softdevice::ble::central::{ScanConfig, ScanError};
use nrf_softdevice::ble::peripheral::AdvertiseError;
use nrf_softdevice::ble::{central, peripheral};
use nrf_softdevice::Softdevice;

pub struct SoftdeviceAdvertisingBearer {
    sd: &'static Softdevice,
}

impl SoftdeviceAdvertisingBearer {
    pub fn new(sd: &'static Softdevice) -> Self {
        Self { sd }
    }
}

impl AdvertisingBearer for SoftdeviceAdvertisingBearer {
    type TransmitFuture<'m> = impl Future<Output = Result<(), BearerError>> + 'm;

    fn transmit<'m>(&'m self, message: &'m Vec<u8, PB_ADV_MTU>) -> Self::TransmitFuture<'m> {
        async move {
            let adv = peripheral::NonconnectableAdvertisement::NonscannableUndirected {
                adv_data: message,
            };

            if let Err(err) = peripheral::advertise(
                self.sd,
                adv,
                &peripheral::Config {
                    max_events: Some(1),
                    timeout: Some(5),
                    interval: 200,
                    ..Default::default()
                },
            )
            .await
            {
                match err {
                    AdvertiseError::Timeout => Ok(()),
                    AdvertiseError::NoFreeConn => Err(BearerError::InsufficientResources),
                    AdvertiseError::Raw(_raw) => Err(BearerError::TransmissionFailure),
                }
            } else {
                Ok(())
            }
        }
    }

    type ReceiveFuture<'m> = impl Future<Output=Result<Vec<u8, PB_ADV_MTU>, BearerError>> + 'm
    where
    Self: 'm;

    fn receive(&self) -> Self::ReceiveFuture<'_> {
        async move {
            let config = ScanConfig {
                active: false,
                interval: 50,
                window: 100,
                ..Default::default()
            };
            loop {
                let result = central::scan::<_, Vec<u8, PB_ADV_MTU>>(self.sd, &config, |event| {
                    let data = event.data;
                    if data.len as usize > PB_ADV_MTU {
                        return None;
                    }
                    let data = unsafe { &*slice_from_raw_parts(data.p_data, data.len as usize) };
                    if data.len() >= 2 && (data[1] == PB_ADV || data[1] == MESH_MESSAGE) {
                        Some(Vec::from_slice(data).unwrap())
                    } else {
                        None
                    }
                })
                .await;

                match result {
                    Ok(data) => {
                        return Ok(data);
                    }
                    Err(err) => {
                        match err {
                            ScanError::Timeout => { /* ignore, loop */ }
                            ScanError::Raw(_) => {
                                return Err(BearerError::Unspecified);
                            }
                        }
                    }
                }
            }
        }
    }
}

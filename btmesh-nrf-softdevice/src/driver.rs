use crate::advertising::SoftdeviceAdvertisingBearer;
use crate::gatt::SoftdeviceGattBearer;
use crate::rng::SoftdeviceRng;
use btmesh_common::Uuid;
use btmesh_driver::stack::interface::NetworkInterfaces;
use btmesh_driver::stack::provisioned::network::DeviceInfo;
use btmesh_driver::stack::provisioned::secrets::Secrets;
use btmesh_driver::stack::provisioned::sequence::Sequence;
use btmesh_driver::stack::provisioned::NetworkState;
use btmesh_driver::{Driver, DriverError};
use btmesh_pdu::provisioning::Capabilities;
use core::mem;
use nrf_softdevice::{raw, Flash, Softdevice};

pub struct NrfSoftdeviceDriverBuilder {
    sd: &'static Softdevice,
}

impl NrfSoftdeviceDriverBuilder {
    pub fn new(name: &'static str) -> Self {
        Self {
            sd: Self::new_sd(name),
        }
    }

    fn new_sd(device_name: &'static str) -> &'static Softdevice {
        let config = nrf_softdevice::Config {
            clock: Some(raw::nrf_clock_lf_cfg_t {
                source: raw::NRF_CLOCK_LF_SRC_RC as u8,
                rc_ctiv: 4,
                rc_temp_ctiv: 2,
                accuracy: 7,
            }),
            conn_gap: Some(raw::ble_gap_conn_cfg_t {
                conn_count: 1,
                event_length: 24,
            }),
            conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 517 }),
            gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t {
                attr_tab_size: 32768,
            }),
            gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
                adv_set_count: 1,
                periph_role_count: 2,
                central_role_count: 2,
                central_sec_count: 2,
                _bitfield_1: Default::default(),
            }),
            gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
                p_value: device_name.as_ptr() as *const u8 as _,
                current_len: device_name.len() as u16,
                max_len: device_name.len() as u16,
                write_perm: unsafe { mem::zeroed() },
                _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
                    raw::BLE_GATTS_VLOC_STACK as u8,
                ),
            }),

            ..Default::default()
        };
        let sd = Softdevice::enable(&config);
        sd
    }

    fn sd(&self) -> &'static Softdevice {
        self.sd
    }

    fn advertising_bearer(&self) -> SoftdeviceAdvertisingBearer {
        SoftdeviceAdvertisingBearer::new(self.sd)
    }

    fn gatt_bearer(&self) -> SoftdeviceGattBearer {
        SoftdeviceGattBearer::new(self.sd)
    }

    fn rng(&self) -> SoftdeviceRng {
        SoftdeviceRng::new(self.sd)
    }

    fn flash(&self) -> Flash {
        Flash::take(self.sd)
    }
}

struct NrfSoftdeviceDriver<N: NetworkInterfaces> {
    driver: Driver<N, SoftdeviceRng>,
}

impl<N: NetworkInterfaces> NrfSoftdeviceDriver<N> {
    pub fn new_unprovisioned(
        network: N,
        rng: SoftdeviceRng,
        capabilities: Capabilities,
        uuid: Uuid,
    ) -> Self {
        Self {
            driver: Driver::new_unprovisioned(network, rng, capabilities, uuid),
        }
    }

    pub fn new_provisioned(
        network: N,
        rng: SoftdeviceRng,
        device_info: DeviceInfo,
        secrets: Secrets,
        network_state: NetworkState,
        sequence: Sequence,
        capabilities: Capabilities,
    ) -> Self {
        Self {
            driver: Driver::new_provisioned(
                network,
                rng,
                device_info,
                secrets,
                network_state,
                sequence,
                capabilities,
            ),
        }
    }

    pub async fn run(&mut self) -> Result<(), DriverError> {
        self.driver.run().await
    }
}

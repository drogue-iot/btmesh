use crate::advertising::SoftdeviceAdvertisingBearer;
use crate::gatt::SoftdeviceGattBearer;
use crate::rng::SoftdeviceRng;
use btmesh_driver::stack::interface::{
    AdvertisingAndGattNetworkInterfaces, AdvertisingOnlyNetworkInterfaces, NetworkInterfaces,
};
use btmesh_driver::storage::flash::FlashBackingStore;
use btmesh_driver::{BluetoothMeshDriver, Driver, DriverError};
use btmesh_pdu::provisioning::Capabilities;
use core::future::Future;
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
        Softdevice::enable(&config)
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

pub struct NrfSoftdeviceDriver<N: NetworkInterfaces> {
    driver: Driver<N, SoftdeviceRng, FlashBackingStore<Flash>>,
}

impl<N: NetworkInterfaces> NrfSoftdeviceDriver<N> {
    pub fn new(
        network: N,
        rng: SoftdeviceRng,
        backing_store: FlashBackingStore<Flash>,
        capabilities: Capabilities,
    ) -> Self {
        Self {
            driver: Driver::new(network, rng, backing_store, capabilities),
        }
    }

    pub async fn run(&mut self) -> Result<(), DriverError> {
        self.driver.run().await
    }
}

pub struct NrfSoftdeviceAdvertisingOnlyDriver(
    NrfSoftdeviceDriver<AdvertisingOnlyNetworkInterfaces<SoftdeviceAdvertisingBearer>>,
);

impl NrfSoftdeviceAdvertisingOnlyDriver {
    pub fn new(
        name: &'static str,
        capabilities: Capabilities,
        base_address: u32,
        sequence_threshold: u32,
    ) -> Self {
        let builder = NrfSoftdeviceDriverBuilder::new(name);
        let rng = builder.rng();
        let backing_store =
            FlashBackingStore::new(builder.flash(), base_address, sequence_threshold);
        let adv_bearer = builder.advertising_bearer();

        let network = AdvertisingOnlyNetworkInterfaces::new(adv_bearer);

        Self(NrfSoftdeviceDriver::new(
            network,
            rng,
            backing_store,
            capabilities,
        ))
    }

    pub async fn run(&mut self) -> Result<(), DriverError> {
        self.0.run().await
    }
}

impl BluetoothMeshDriver for NrfSoftdeviceAdvertisingOnlyDriver {
    type RunFuture<'f> = impl Future<Output=Result<(), DriverError>> + 'f
    where
    Self: 'f;

    fn run(&mut self) -> Self::RunFuture<'_> {
        self.0.run()
    }
}

pub struct NrfSoftdeviceAdvertisingAndGattDriver(
    NrfSoftdeviceDriver<
        AdvertisingAndGattNetworkInterfaces<SoftdeviceAdvertisingBearer, SoftdeviceGattBearer, 66>,
    >,
);

impl NrfSoftdeviceAdvertisingAndGattDriver {
    pub fn new(
        name: &'static str,
        capabilities: Capabilities,
        base_address: u32,
        sequence_threshold: u32,
    ) -> Self {
        let builder = NrfSoftdeviceDriverBuilder::new(name);
        let rng = builder.rng();
        let backing_store =
            FlashBackingStore::new(builder.flash(), base_address, sequence_threshold);
        let adv_bearer = builder.advertising_bearer();
        let gatt_bearer = builder.gatt_bearer();

        let network = AdvertisingAndGattNetworkInterfaces::new(adv_bearer, gatt_bearer);

        Self(NrfSoftdeviceDriver::new(
            network,
            rng,
            backing_store,
            capabilities,
        ))
    }

    pub async fn run(&mut self) -> Result<(), DriverError> {
        self.0.run().await
    }
}

impl BluetoothMeshDriver for NrfSoftdeviceAdvertisingAndGattDriver {
    type RunFuture<'f> = impl Future<Output=Result<(), DriverError>> + 'f
    where
    Self: 'f;

    fn run(&mut self) -> Self::RunFuture<'_> {
        self.0.run()
    }
}
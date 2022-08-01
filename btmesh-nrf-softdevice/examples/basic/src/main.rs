#![no_std]
#![no_main]
#![macro_use]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

use embassy::executor::Spawner;
use embassy_nrf::Peripherals;

use embassy_nrf::config::Config;
use embassy_nrf::interrupt::Priority;

use btmesh_nrf_softdevice::driver::NrfSoftdeviceAdvertisingOnlyDriver;

use btmesh_pdu::provisioning::Algorithms;
use btmesh_pdu::provisioning::Capabilities;
use btmesh_pdu::provisioning::InputOOBActions;
use btmesh_pdu::provisioning::OOBSize;
use btmesh_pdu::provisioning::OutputOOBActions;
use btmesh_pdu::provisioning::PublicKeyType;
use btmesh_pdu::provisioning::StaticOOBType;

use defmt_rtt as _;
use panic_probe as _;

mod device;

extern "C" {
    static __storage: u8;
}

#[embassy::main(config = "config()")]
async fn main(_spawner: Spawner, _p: Peripherals) {
    let capabilities = Capabilities {
        number_of_elements: 1,
        algorithms: Algorithms::default(),
        public_key_type: PublicKeyType::default(),
        static_oob_type: StaticOOBType::default(),
        output_oob_size: OOBSize::MaximumSize(4),
        output_oob_action: OutputOOBActions::default(),
        input_oob_size: OOBSize::MaximumSize(4),
        input_oob_action: InputOOBActions::default(),
    };


    let mut driver = NrfSoftdeviceAdvertisingOnlyDriver::new(
        "drogue",
        capabilities,
        unsafe { &__storage as *const u8 as u32 },
        100,
    );
    driver.run().await.unwrap();
}

// Application must run at a lower priority than softdevice
fn config() -> Config {
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = Priority::P2;
    config.time_interrupt_priority = Priority::P2;
    config
}

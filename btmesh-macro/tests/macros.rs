#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use btmesh_device::{BluetoothMeshModel, BluetoothMeshModelContext};
use btmesh_macro::*;
use btmesh_models::generic::onoff::GenericOnOffServer;

#[test]
fn test_element_macro() {
    #[element(location = "internal")]
    pub struct MyElem {
        config: MyModel,
    }

    pub struct MyModel;

    impl BluetoothMeshModel<GenericOnOffServer> for MyModel {
        async fn run<C: BluetoothMeshModelContext<GenericOnOffServer>>(
            &mut self,
            _: C,
        ) -> Result<(), ()> {
            loop {}
        }
    }
}

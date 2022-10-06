#![feature(type_alias_impl_trait)]

use btmesh_device::{BluetoothMeshModel, BluetoothMeshModelContext};
use btmesh_macro::*;
use btmesh_models::generic::onoff::GenericOnOffServer;
use core::future::Future;

#[test]
fn test_element_macro() {
    #[element(location = "internal")]
    pub struct MyElem {
        config: MyModel,
    }

    pub struct MyModel;

    impl BluetoothMeshModel<GenericOnOffServer> for MyModel {
        type RunFuture<'f, C>  = impl Future<Output=Result<(),()>> + 'f
            where
            Self: 'f, C: BluetoothMeshModelContext<GenericOnOffServer> + 'f;

        fn run<'run, C: BluetoothMeshModelContext<GenericOnOffServer> + 'run>(
            &'run mut self,
            _: C,
        ) -> Self::RunFuture<'_, C> {
            async move { loop {} }
        }
    }
}

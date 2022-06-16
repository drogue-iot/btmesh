
#[derive(Copy, Clone)]
pub struct DeviceKey {
    device_key: [u8;16]
}

impl DeviceKey {
    pub(crate) fn device_key(&self) -> [u8; 16] {
        self.device_key
    }

}
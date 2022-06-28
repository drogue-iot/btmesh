use crate::address::{Address, UnicastAddress};
use crate::mic::SzMic;
use crate::{IvIndex, Seq};
use core::ops::Deref;

struct NonceType(u8);

pub enum Nonce {
    Network(NetworkNonce),
    Application(ApplicationNonce),
    Device(DeviceNonce),
    Proxy(ProxyNonce),
}

pub struct NetworkNonce([u8; 13]);

impl Deref for NetworkNonce {
    type Target = [u8; 13];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl NetworkNonce {
    const NONCE_TYPE: NonceType = NonceType(0x00);

    pub fn new(ctl_ttl: u8, seq: Seq, src: UnicastAddress, iv_index: IvIndex) -> Self {
        let mut nonce = [0; 13];
        nonce[0] = Self::NONCE_TYPE.0;
        nonce[1] = ctl_ttl;

        let seq = seq.to_be_bytes();
        nonce[2] = seq[1];
        nonce[3] = seq[2];
        nonce[4] = seq[3];

        let src = src.as_bytes();
        nonce[5] = src[0];
        nonce[6] = src[1];

        nonce[7] = 0x00;
        nonce[8] = 0x00;

        let iv_index = iv_index.to_be_bytes();
        nonce[9] = iv_index[0];
        nonce[10] = iv_index[1];
        nonce[11] = iv_index[2];
        nonce[12] = iv_index[3];

        Self(nonce)
    }

    pub fn into_bytes(self) -> [u8; 13] {
        self.0
    }
}

fn build_nonce(
    nonce_type: NonceType,
    aszmic: SzMic,
    seq: Seq,
    src: UnicastAddress,
    dst: Address,
    iv_index: IvIndex,
) -> [u8; 13] {
    let mut nonce = [0; 13];
    nonce[0] = nonce_type.0;
    match aszmic {
        SzMic::Bit32 => {
            nonce[1] = 0b00000000;
        }
        SzMic::Bit64 => {
            nonce[1] = 0b10000000;
        }
    }

    let seq = seq.to_be_bytes();
    nonce[2] = seq[1];
    nonce[3] = seq[2];
    nonce[4] = seq[3];

    let src = src.as_bytes();
    nonce[5] = src[0];
    nonce[6] = src[1];

    let dst = dst.as_bytes();
    nonce[7] = dst[0];
    nonce[8] = dst[1];

    let iv_index = iv_index.to_be_bytes();
    nonce[9] = iv_index[0];
    nonce[10] = iv_index[1];
    nonce[11] = iv_index[2];
    nonce[12] = iv_index[3];

    nonce
}

#[derive(Copy, Clone)]
pub struct ApplicationNonce([u8; 13]);

impl ApplicationNonce {
    const NONCE_TYPE: NonceType = NonceType(0x01);

    pub fn new(
        aszmic: SzMic,
        seq: Seq,
        src: UnicastAddress,
        dst: Address,
        iv_index: IvIndex,
    ) -> Self {
        Self(build_nonce(
            Self::NONCE_TYPE,
            aszmic,
            seq,
            src,
            dst,
            iv_index,
        ))
    }
}

impl Deref for ApplicationNonce {
    type Target = [u8; 13];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DeviceNonce([u8; 13]);

impl DeviceNonce {
    const NONCE_TYPE: NonceType = NonceType(0x02);

    pub fn new(
        aszmic: SzMic,
        seq: Seq,
        src: UnicastAddress,
        dst: Address,
        iv_index: IvIndex,
    ) -> Self {
        Self(build_nonce(
            Self::NONCE_TYPE,
            aszmic,
            seq,
            src,
            dst,
            iv_index,
        ))
    }
}

impl Deref for DeviceNonce {
    type Target = [u8; 13];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct ProxyNonce([u8; 13]);

#[cfg(test)]
mod test {
    use crate::address::UnicastAddress;
    use crate::crypto::nonce::NetworkNonce;
    use crate::{IvIndex, Seq};

    #[test]
    fn network_nonce() {
        // Test Message #1, Network PDU
        let expected = [
            0x00, 0x80, 0x00, 0x00, 0x01, 0x12, 0x01, 0x00, 0x00, 0x12, 0x34, 0x56, 0x78,
        ];

        let ctl_ttl = 0x80;
        let seq = Seq::parse(0x000001).unwrap();
        let src = UnicastAddress::parse([0x12, 0x01]).unwrap();
        let iv_index = IvIndex::parse(&[0x12, 0x34, 0x56, 0x78]).unwrap();

        let result = NetworkNonce::new(ctl_ttl, seq, src, iv_index);

        assert_eq!(expected, result.into_bytes())
    }
}

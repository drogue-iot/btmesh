use crate::Nid;

pub trait NetworkKeyHandle {
    fn nid(&self) -> Nid;
}

pub trait Secrets {
    type NetworkKeyHandle: NetworkKeyHandle;
}
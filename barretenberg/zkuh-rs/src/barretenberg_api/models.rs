use crate::barretenberg_api::utils::SerializeBuffer;
use std::ffi::c_void;

pub type Ptr = *mut c_void;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Fr {
    pub data: [u8; 32],
}

impl SerializeBuffer for Fr {
    fn to_buffer(&self) -> Vec<u8> {
        self.data.to_vec()
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Fq {
    pub data: [u8; 32],
}

impl SerializeBuffer for Fq {
    fn to_buffer(&self) -> Vec<u8> {
        self.data.to_vec()
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Point {
    pub x: Fr,
    pub y: Fr,
}

impl SerializeBuffer for Point {
    fn to_buffer(&self) -> Vec<u8> {
        self.x
            .to_buffer()
            .into_iter()
            .chain(self.y.to_buffer())
            .collect()
    }
}

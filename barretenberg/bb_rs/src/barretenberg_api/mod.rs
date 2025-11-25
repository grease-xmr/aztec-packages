#![allow(non_snake_case)]
pub mod acir;
pub mod bbapi;
pub mod models;

#[allow(unused)]
mod untested;

#[cfg(test)]
pub mod tests;

mod bindgen {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]

    // This matches bindgen::Builder output
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub(crate) mod utils;

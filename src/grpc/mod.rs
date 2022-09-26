#![allow(unknown_lints)]
#![allow(clippy::all)]
#![allow(dead_code)]

pub mod authentication {
    include!(concat!(env!("OUT_DIR"), "/authentication.rs"));
}

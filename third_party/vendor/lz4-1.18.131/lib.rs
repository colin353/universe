#![allow(non_camel_case_types)]

pub mod liblz4;

mod decoder;
mod encoder;
mod lz4_sys;

pub mod block;

pub use crate::decoder::Decoder;
pub use crate::encoder::Encoder;
pub use crate::encoder::EncoderBuilder;
pub use crate::liblz4::version;
pub use crate::liblz4::BlockMode;
pub use crate::liblz4::BlockSize;
pub use crate::liblz4::ContentChecksum;

type c_char = i8;
type size_t = usize;
type c_int = i32;
type c_uint = u32;

#[derive(Debug)]
pub enum c_void {}

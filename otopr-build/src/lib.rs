//! Build script library for OtoPr.

use prost_types::FileDescriptorSet;

pub enum MessageConfig {
    /// Generates "borrowed" instances, since we just need the data for encoding.
    /// 
    /// 
    EncodeOnly,
    /// Generate "owned" instances, since we are only decoding.
    /// 
    /// Does not use zero-copy deserialization.
    DecodeOnly,
    DoNotGenerate,
}

pub struct Config<F> {
    msg: F,
}

pub fn generate_sources(set: FileDescriptorSet) -> String {
    "".into()
}
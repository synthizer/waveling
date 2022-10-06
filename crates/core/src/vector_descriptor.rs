//! A vector descriptor describes the shape of an audio output.
//!
//! Vectors are the results of nodes and a single frame stored in a state.

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd, Copy, Clone, Hash)]
pub enum PrimitiveType {
    Bool,
    I64,

    /// Most common type for samples.
    F32,

    /// Used for things in which an F32 is too imprecise, for example biquad coefficients.
    F64,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct VectorDescriptor {
    pub primitive: PrimitiveType,
    pub width: u64,
}

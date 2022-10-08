//! A vector descriptor describes the shape of an audio output.
//!
//! Vectors are the results of nodes and a single frame stored in a state.
use std::fmt::Display;

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd, Copy, Clone, Hash, strum::Display)]
#[strum(serialize_all = "snake_case")]
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

impl VectorDescriptor {
    pub fn new_bool(width: u64) -> Self {
        Self {
            primitive: PrimitiveType::Bool,
            width,
        }
    }

    pub fn new_i64(width: u64) -> Self {
        Self {
            primitive: PrimitiveType::I64,
            width,
        }
    }

    pub fn new_f32(width: u64) -> Self {
        Self {
            primitive: PrimitiveType::F32,
            width,
        }
    }

    pub fn new_f64(width: u64) -> Self {
        Self {
            primitive: PrimitiveType::F64,
            width,
        }
    }
}

impl Display for VectorDescriptor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.width == 1 {
            write!(formatter, "{}", self.primitive)?;
        } else {
            write!(formatter, "{}<{}>", self.primitive, self.width)?;
        }

        Ok(())
    }
}

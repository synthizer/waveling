use crate::*;

/// Data types for nodes.
#[derive(
    Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, derive_more::Display, derive_more::IsVariant,
)]
pub enum DataType {
    /// This is the "no type"/unit/void/!; nodes with this type do not produce data.
    Never,

    /// nodes with this type produce vectors.
    Vector(VectorDescriptor),
}

impl DataType {
    pub fn new_vector(prim: PrimitiveType, width: u64) -> Self {
        Self::Vector(VectorDescriptor::new(prim, width))
    }

    pub fn new_v_bool(width: u64) -> Self {
        Self::new_vector(PrimitiveType::Bool, width)
    }

    pub fn new_v_i64(width: u64) -> Self {
        Self::new_vector(PrimitiveType::I64, width)
    }

    pub fn new_v_f32(width: u64) -> Self {
        Self::new_vector(PrimitiveType::F32, width)
    }

    pub fn new_v_f64(width: u64) -> Self {
        Self::new_vector(PrimitiveType::F64, width)
    }
}

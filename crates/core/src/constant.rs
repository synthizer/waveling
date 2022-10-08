use crate::{PrimitiveType, VectorDescriptor};

/// A vector constant.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Constant {
    Bool(Vec<bool>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
}

impl Constant {
    pub fn primitive_type(&self) -> PrimitiveType {
        match self {
            Self::Bool(_) => PrimitiveType::Bool,
            Self::I64(_) => PrimitiveType::I64,
            Self::F32(_) => PrimitiveType::F32,
            Self::F64(_) => PrimitiveType::F64,
        }
    }

    pub fn width(&self) -> u64 {
        let w = match self {
            Self::Bool(v) => v.len(),
            Self::I64(v) => v.len(),
            Self::F32(v) => v.len(),
            Self::F64(v) => v.len(),
        };

        w as u64
    }

    pub fn vector_descriptor(&self) -> VectorDescriptor {
        VectorDescriptor {
            primitive: self.primitive_type(),
            width: self.width(),
        }
    }
}

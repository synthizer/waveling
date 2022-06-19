//! Description of the type system.
//!
//!  We have a set of primitive types, which model scalars as 1-unit vectores plus some special casing in the backends,
//!  and possibly a wrapping buffer of a known size.  We model this as 2 dimensions, call anything with the buffer
//!  length of 1 a primitive, call anything with a vector width of 1 and a buffer length of 1 a scalar.  This makes
//!  sense because it does actually make some sense to read from the 1-element buffer scalar at position 0, for example;
//!  inefficient if lowered that way, but sensible nonetheless.
use std::num::NonZeroU64;

use anyhow::{anyhow, Result};

/// Primitive kinds.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, derive_more::Display)]
pub enum Primitive {
    I32,
    I64,
    F32,
    F64,
    Bool,
}

/// Description of a type.
#[derive(Debug, Eq, PartialEq, Copy, Clone, derive_more::Display)]
#[display(fmt = "{}<{}, {}>", primitive, vector_width, buffer_length)]
pub struct Type {
    /// The underlying primitive type.
    primitive: Primitive,

    /// The width of the vector. 1 means scalar.
    vector_width: NonZeroU64,

    /// The length of the buffer being described. 1 means normal variable.
    buffer_length: NonZeroU64,
}

impl Type {
    pub fn new(primitive: Primitive, vector_width: u64, buffer_length: u64) -> Result<Type> {
        let vector_width = NonZeroU64::new(vector_width).ok_or_else(|| {
            anyhow!("Internal error: attempt to construct a type with a zero vector width")
        })?;
        let buffer_length = NonZeroU64::new(buffer_length).ok_or_else(|| {
            anyhow!("Internal error: attempt to construct type with a zero buffer length")
        })?;

        Ok(Type {
            primitive,
            vector_width,
            buffer_length,
        })
    }

    pub fn new_vector(primitive: Primitive, vector_width: u64) -> Result<Type> {
        Type::new(primitive, vector_width, 1)
    }

    pub fn new_scalar(primitive: Primitive) -> Result<Type> {
        Type::new(primitive, 1, 1)
    }

    /// True if this is a scalar, aka a buffer of length 1 of vector 1.
    pub fn is_scalar(&self) -> bool {
        self.vector_width.get() == 1 && self.buffer_length.get() == 1
    }

    /// True if this is a vector, aka width > 1 but not buffer length > 1.
    pub fn is_vector(&self) -> bool {
        self.vector_width.get() > 1 && self.buffer_length.get() == 1
    }

    /// This describes a buffer: the vector width can be anything, but the buffer length is more than 1.
    pub fn is_buffer(&self) -> bool {
        self.buffer_length.get() > 1
    }

    pub fn get_primitive(&self) -> Primitive {
        self.primitive
    }

    pub fn get_vector_width(&self) -> u64 {
        self.vector_width.get()
    }

    pub fn get_buffer_length(&self) -> u64 {
        self.buffer_length.get()
    }
}

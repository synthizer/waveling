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

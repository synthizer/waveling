use crate::VectorDescriptor;

/// A state is a writable memory location, usually read with modulus as a delay line.
#[derive(Debug)]
pub struct State {
    /// The kind of data this state holds.
    pub vector: VectorDescriptor,

    /// The length of this state.
    pub length: u64,
}

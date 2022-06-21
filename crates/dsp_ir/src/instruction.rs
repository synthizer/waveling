use crate::context::*;

/// The instruction enum.
///
/// The convenient interface, which also verifies that invariants hold, is the [crate::inst_builder] module.  The
/// documentation on this enum is authoritative as to these invariants, but this enum is admittedly difficult to work
/// with.
///
/// Instructions have zero or more inputs and always exactly one output.
///
/// Unless otherwise documented, instructions take either two vectors of the same type and length, or a vector and a
/// scalar.  They don't care which is which, for convenience.
///
/// The fast trigonometric instructions are only guaranteed to be accurate  on the range `-2pi` to `2pi` inclusive.  How
/// accurate they are is still up in the air.  They must also be executed on an f32 or f64 type.
pub enum Instruction {
    /// Addition.
    Add {
        output: ValueRef,
        left: ValueRef,
        right: ValueRef,
    },

    /// Subtraction.
    Sub {
        output: ValueRef,
        left: ValueRef,
        right: ValueRef,
    },

    /// Multiplication
    Mul {
        output: ValueRef,
        left: ValueRef,
        right: ValueRef,
    },

    /// Division
    Div {
        output: ValueRef,
        left: ValueRef,
        right: ValueRef,
    },

    /// Power.
    Pow {
        output: ValueRef,
        base: ValueRef,
        exponent: ValueRef,
    },

    FastSin {
        output: ValueRef,
        input: ValueRef,
    },

    FastCos {
        output: ValueRef,
        input: ValueRef,
    },

    FastTan {
        output: ValueRef,
        input: ValueRef,
    },

    /// Hyperbolic sin.
    FastSinh {
        output: ValueRef,
        input: ValueRef,
    },

    /// Hyperbolic cosine.
    FastCosh {
        output: ValueRef,
        input: ValueRef,
    },

    /// Hyperbolic tangent.
    FastTanh {
        output: ValueRef,
        input: ValueRef,
    },

    Min {
        output: ValueRef,
        left: ValueRef,
        right: ValueRef,
    },

    Max {
        output: ValueRef,
        left: ValueRef,
        right: ValueRef,
    },

    Clamp {
        output: ValueRef,
        input: ValueRef,
        lower: ValueRef,
        upper: ValueRef,
    },

    ToF32 {
        input: ValueRef,
        output: ValueRef,
    },

    ToF64 {
        input: ValueRef,
        output: ValueRef,
    },

    /// Perform modulus on two guaranteed-to-be positive values.
    ///
    /// Both inputs must be of the same type.  Does either integer modulus or floating point modulus.
    ModPositive {
        output: ValueRef,
        input: ValueRef,
        divisor: ValueRef,
    },

    /// Read a state at a given index.
    ///
    /// Index should be an integral scalar.
    ///
    /// Note that scalars are 1-length buffers.  When emitting  index reading for scalars, use 0.
    ReadState {
        output: ValueRef,
        state: StateRef,
        index: ValueRef,
    },

    /// Read a state, but relative to the modulus of the current sample index.
    ///
    /// This is used to efficiently implement ringbuffers: the current index is `sample % length`, and the read is
    /// relative to that.  The difference between this instruction and an addition followed by a read is that the
    /// counter is guaranteed to be shared.
    ReadStateRelative {
        output: ValueRef,
        state: StateRef,
        index: ValueRef,
    },

    WriteState {
        state: StateRef,
        input: ValueRef,
        index: ValueRef,
    },

    WriteStateRelative {
        state: StateRef,
        input: ValueRef,
        index: ValueRef,
    },

    /// Read the current time, in samples.
    ReadTimeSamples {
        output: ValueRef,
    },

    /// Read the current time, in seconds.
    ReadTimeSeconds {
        output: ValueRef,
    },

    /// Read an input of the program, at the current sample index.
    ReadInput {
        output: ValueRef,
        input: usize,
    },

    /// Write an output of the current program.
    WriteOutput {
        input: ValueRef,
        index: usize,
    },

    /// Read a property of the program.
    ///
    /// Properties are always f64 (and so internally converted to integers by user code if that's what they need).
    /// Currently we additionally place the constraint that properties are scalar.
    ReadProperty {
        output: ValueRef,
        property: usize,
    },
}

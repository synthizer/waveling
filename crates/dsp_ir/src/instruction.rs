use crate::context::ValueRef;

/// The instruction enum.
///
/// The convenient interface, which also verifies that invariants hold, is the [crate::InstBuilder].  The documentation
/// on this type is authoritative as to these invariants, but this enum is admittedly difficult to work with.
///
/// Instructions have zero or more inputs and always exactly one output.
///
/// Unless otherwise documented, instructions take either two vectors of the same type and length, or a vector and a
/// scalar.  They don't care which is which, for convenience.
///
/// The fast trigonometric instructions are only guaranteed to be accurate  on the range `-2pi` to `2pi` inclusive.  How
/// accurate they are is still up in the air.
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
    ///
    /// As a special case, the exponent may be of an integral type when the base is of a floating point typee, which may
    /// be used to implement more efficiently on some backends when the exponent is a whole number.
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
}
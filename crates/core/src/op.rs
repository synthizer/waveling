use crate::{Constant, PrimitiveType};

/// Binary operations that we support.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// Kinds of operation associated with a node.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Op {
    Constant(Constant),

    Negate,
    BinOp(BinOp),

    /// Read the given input.
    ReadInput(usize),

    /// Write the given output.
    WriteOutput(usize),

    /// Read a property.
    ReadProperty(usize),

    /// Write to a given state.
    ///
    /// The input is the value to write.
    ///
    /// The location must be an integral type.
    ReadState {
        state: usize,

        /// If true, this read should be mod the state's length.
        modulus: bool,
    },

    /// Write to a state.
    ///
    /// The 0th input is the value to write and the 1st input the location.
    WriteState {
        state: usize,

        /// If true, take the given location as mod the state length.
        modulus: bool,
    },

    /// Read the clock, an i64 integer that increments every sample.
    Clock,

    /// Cast the only input to the given primitive type.
    ///
    /// We don't perform implicit casts because it is important to always know where they happen.
    Cast(PrimitiveType),

    /// The synthetic start node is used to have a single entry node, rather than n entry nodes.
    ///
    /// Doesn't carry data.
    Start,

    /// The final node is used to have a single point at which the program ends.
    ///
    /// Doesn't care about the input types and outputs nothing.
    ///
    /// This gives us a place to hook side-effecting operations which are not related to outputs, for example writing
    /// states.
    Final,
}

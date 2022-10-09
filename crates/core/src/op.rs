use std::borrow::Cow;

use crate::{Constant, PrimitiveType};

/// Binary operations that we support.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, derive_more::IsVariant)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// Kinds of operation associated with a node.
#[derive(Clone, Debug, PartialEq, PartialOrd, derive_more::IsVariant)]
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

    /// Read the sample rate.
    Sr,

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

/// A descriptor for an operation, which describes the inputs and outputs for the type checker and opptimization passes.
#[derive(Clone, Debug)]
pub struct OpDescriptor {
    /// Is this operator commutative?
    ///
    /// If so, then `a op b == b op a`.  We use a relaxed model that assumes fp ops are commutative.
    pub commutative: bool,

    pub inputs: Cow<'static, [InputDescriptor]>,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, derive_more::IsVariant)]
enum InputKind {
    /// This edge carries data.
    ///
    /// Edges which carry data must type check.
    Data,

    /// This edge is a pure dependency.
    ///
    /// these are used to, for example, connect everything that needs to be connected to the final node.  Such edges
    /// don't care about types.
    PureDependency,
}
#[derive(Clone, Debug)]
pub struct InputDescriptor {
    input_kind: InputKind,

    /// We assume all primitive types are allowed unless otherwise specified; this is the list of denied primitive
    /// types.
    ///
    /// For example, we can't apply arithmetic binary operations to booleans.
    denied_primitives: Option<Cow<'static, [PrimitiveType]>>,
}

fn binop_to_descriptor(o: BinOp) -> OpDescriptor {
    OpDescriptor {
        commutative: [BinOp::Add, BinOp::Mul].contains(&o),
        inputs: Cow::Borrowed(&[InputDescriptor {
            input_kind: InputKind::Data,
            denied_primitives: Some(Cow::Borrowed(&[PrimitiveType::Bool])),
        }]),
    }
}

impl Op {
    pub fn get_descriptor(&self) -> Cow<'static, OpDescriptor> {
        match *self {
            // All of our boring ones with no inputs.
            Op::Start
            | Op::ReadInput(_)
            | Op::ReadProperty(_)
            | Op::Constant(_)
            | Op::ReadState { .. }
            | Op::Clock
            | Op::Sr => Cow::Borrowed(&OpDescriptor {
                commutative: false,
                inputs: Cow::Borrowed(&[]),
            }),
            Op::BinOp(o) => Cow::Owned(binop_to_descriptor(o)),
            Op::Negate => Cow::Owned(OpDescriptor {
                commutative: false,
                inputs: Cow::Borrowed(&[InputDescriptor {
                    input_kind: InputKind::Data,
                    denied_primitives: Some(Cow::Borrowed(&[PrimitiveType::Bool])),
                }]),
            }),
            // The difference from Negate is that cast allows all inputs.
            Op::Cast(_) => Cow::Borrowed(&OpDescriptor {
                commutative: false,
                inputs: Cow::Borrowed(&[InputDescriptor {
                    input_kind: InputKind::Data,
                    denied_primitives: None,
                }]),
            }),
            Op::WriteOutput { .. } | Op::WriteState { .. } => Cow::Borrowed(&OpDescriptor {
                commutative: false,
                inputs: Cow::Borrowed(&[InputDescriptor {
                    input_kind: InputKind::Data,
                    denied_primitives: None,
                }]),
            }),
            // Difference here is that final inputs are pure dependerncies.
            Op::Final => Cow::Borrowed(&OpDescriptor {
                commutative: false,
                inputs: Cow::Borrowed(&[InputDescriptor {
                    input_kind: InputKind::PureDependency,
                    denied_primitives: None,
                }]),
            }),
        }
    }
}

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

/// What kind of implicit edges does this operation have?
///
/// This is used to feed setup of the edges from the start and final nodes rather than having logic scattered all over;
/// declarative is easier to reason about.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, derive_more::IsVariant)]
pub enum ImplicitEdgeKind {
    /// All edges for this node must be declared b the user.
    NoImplicitEdges,

    /// This node implicitly conects to the start node.
    Start,

    /// This node implicitly connecs to the final node.
    Final,
}

/// A descriptor for an operation, which describes the inputs and outputs for the type checker and opptimization passes.
#[derive(Clone, Debug)]
pub struct OpDescriptor {
    /// Is this operator commutative?
    ///
    /// If so, then `a op b == b op a`.  We use a relaxed model that assumes fp ops are commutative.
    pub commutative: bool,

    pub implicit_edges: ImplicitEdgeKind,

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
        implicit_edges: ImplicitEdgeKind::NoImplicitEdges,
        inputs: Cow::Borrowed(&[InputDescriptor {
            input_kind: InputKind::Data,
            denied_primitives: Some(Cow::Borrowed(&[PrimitiveType::Bool])),
        }]),
    }
}

impl Op {
    pub fn get_descriptor(&self) -> Cow<'static, OpDescriptor> {
        match *self {
            Op::Start => Cow::Borrowed(&OpDescriptor {
                commutative: false,
                // This is the start node, which doesn't get edges to itself.
                implicit_edges: ImplicitEdgeKind::NoImplicitEdges,
                inputs: Cow::Borrowed(&[]),
            }),
            // these must have a connection from the start node.
            Op::ReadInput(_)
            | Op::ReadProperty(_)
            | Op::Constant(_)
            | Op::ReadState { .. }
            | Op::Clock
            | Op::Sr => Cow::Borrowed(&OpDescriptor {
                commutative: false,
                implicit_edges: ImplicitEdgeKind::Start,
                inputs: Cow::Borrowed(&[InputDescriptor {
                    input_kind: InputKind::PureDependency,
                    denied_primitives: None,
                }]),
            }),
            Op::BinOp(o) => Cow::Owned(binop_to_descriptor(o)),
            Op::Negate => Cow::Owned(OpDescriptor {
                commutative: false,
                implicit_edges: ImplicitEdgeKind::NoImplicitEdges,
                inputs: Cow::Borrowed(&[InputDescriptor {
                    input_kind: InputKind::Data,
                    denied_primitives: Some(Cow::Borrowed(&[PrimitiveType::Bool])),
                }]),
            }),
            // The difference from Negate is that cast allows all inputs.
            Op::Cast(_) => Cow::Borrowed(&OpDescriptor {
                commutative: false,
                implicit_edges: ImplicitEdgeKind::NoImplicitEdges,
                inputs: Cow::Borrowed(&[InputDescriptor {
                    input_kind: InputKind::Data,
                    denied_primitives: None,
                }]),
            }),
            Op::WriteOutput { .. } | Op::WriteState { .. } => Cow::Borrowed(&OpDescriptor {
                commutative: false,
                implicit_edges: ImplicitEdgeKind::Final,
                inputs: Cow::Borrowed(&[InputDescriptor {
                    input_kind: InputKind::Data,
                    denied_primitives: None,
                }]),
            }),
            // Difference here is that final inputs are pure dependerncies, and of course it doesn't have edges to
            // itself.
            Op::Final => Cow::Borrowed(&OpDescriptor {
                commutative: false,
                implicit_edges: ImplicitEdgeKind::NoImplicitEdges,
                inputs: Cow::Borrowed(&[InputDescriptor {
                    input_kind: InputKind::PureDependency,
                    denied_primitives: None,
                }]),
            }),
        }
    }
}

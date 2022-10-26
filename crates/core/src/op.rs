use std::borrow::Cow;

use crate::{Constant, PrimitiveType};

/// Binary operations that we support.
#[derive(
    Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, derive_more::Display, derive_more::IsVariant,
)]
pub enum BinOp {
    #[display(fmt = "+")]
    Add,

    #[display(fmt=:"-")]
    Sub,

    #[display(fmt = "*")]
    Mul,

    #[display(fmt = "/")]
    Div,
}

/// Kinds of operation associated with a node.
#[derive(Clone, Debug, PartialEq, PartialOrd, derive_more::Display, derive_more::IsVariant)]
pub enum Op {
    #[display(fmt = "const({_0})")]
    Constant(Constant),

    Negate,

    BinOp(BinOp),

    /// Read the given input.
    #[display(fmt = "ReadInput({_0})")]
    ReadInput(usize),

    /// Write the given output.
    #[display(fmt = "WriteOutput({_0})")]
    WriteOutput(usize),

    /// Read a property.
    #[display(fmt = "ReadProperty({_0})")]
    ReadProperty(usize),

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
            Op::Start => Cow::Borrowed(&OpDescriptor {
                commutative: false,
                // This is the start node, which doesn't get edges to itself.
                inputs: Cow::Borrowed(&[]),
            }),
            // these must have a connection from the start node.
            Op::ReadInput(_) | Op::ReadProperty(_) | Op::Constant(_) | Op::Clock | Op::Sr => {
                Cow::Borrowed(&OpDescriptor {
                    commutative: false,

                    inputs: Cow::Borrowed(&[InputDescriptor {
                        input_kind: InputKind::PureDependency,
                        denied_primitives: None,
                    }]),
                })
            }
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
            Op::WriteOutput { .. } => Cow::Borrowed(&OpDescriptor {
                commutative: false,

                inputs: Cow::Borrowed(&[InputDescriptor {
                    input_kind: InputKind::Data,
                    denied_primitives: None,
                }]),
            }),
            // Difference here is that final inputs are pure dependerncies, and of course it doesn't have edges to
            // itself.
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

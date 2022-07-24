use anyhow::Result;

use super::*;

/// A reference to a value.
///
/// A [ValueRef] is produced by the execution of an instruction, or directly from a constant.  Loading from "memory" or
/// otherwise getting access to things like buffers and special resources happens via load instructions, and local
/// variables are implicit in our graph-based model.
///
/// [ValueRef]s are produced from contexts.  For convenience they don't use branding lifetimes, but mixing them up
/// between contexts will result in all sorts of oddness.
///
/// Our two kinds of values:
///
/// - Immediates are constants, which will be lowered as raw values where applicable.
/// - Instruction outputs are outputs from instructions, which may in turn be given to other instructions as input.
///   Inour model, every instruction has exactly one output.
///
/// Example flows:
///
/// - 6.0f32 is an immediate.
/// - the output from the biquad filter is (barring constant folding) an instruction output.
/// - Reading from a buffer or global state uses a special load instruction to get from that state to an instruction
///   output, that is, the IR models even single variables that last longer than one program iteration as pointers.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd, Copy, Clone, Hash)]
pub struct ValueRef {
    pub(crate) index: generational_arena::Index,
}

// The rest of this describes internal types which are stored in the context's tables.

pub(crate) enum ValueKind {
    /// It's a constant, with this index into the constants table.
    Constant(super::ConstantRef),

    /// Or otherwise, computed from something.
    Computed,
}

pub(crate) struct ValueDescriptor {
    kind: ValueKind,
    value_type: crate::types::Type,
}

/// Error yielded when resolving a value fails.
#[derive(Debug, thiserror::Error)]
#[error("Resolution failed due to out of range indices in the values table")]
pub struct ValueResolutionFailed;

impl ValueRef {
    fn resolve<'a>(&self, context: &'a Context) -> Result<&'a ValueDescriptor> {
        Ok(context
            .value_arena
            .get(self.index)
            .ok_or(ValueResolutionFailed)?)
    }

    pub fn get_type(&self, context: &Context) -> Result<crate::types::Type> {
        let desc = self.resolve(context)?;
        Ok(desc.value_type)
    }

    pub fn is_constant(&self, context: &Context) -> Result<bool> {
        if let ValueKind::Constant(_) = self.resolve(context)?.kind {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn get_constant(&self, ctx: &Context) -> Result<Option<ConstantRef>> {
        let resolved = self.resolve(ctx)?;
        match resolved.kind {
            ValueKind::Computed => Ok(None),
            ValueKind::Constant(x) => Ok(Some(x)),
        }
    }
}

/// Context methods for values.
impl Context {
    /// Allocate a value suitable for being the output of an instruction.
    pub fn new_value(&mut self, value_type: crate::types::Type) -> ValueRef {
        self.new_value_impl(value_type, ValueKind::Computed)
    }

    /// Create a value with a given constant, by adding said constant to the constants table.
    pub fn new_value_const(
        &mut self,
        value_type: crate::types::Type,
        constant: waveling_const::Constant,
    ) -> ValueRef {
        let nc = self.new_constant(constant);
        self.new_value_const_ref(value_type, nc)
    }

    /// create a value for a constant already in the constants table.
    pub fn new_value_const_ref(
        &mut self,
        value_type: crate::types::Type,
        const_ref: ConstantRef,
    ) -> ValueRef {
        self.new_value_impl(value_type, ValueKind::Constant(const_ref))
    }

    fn new_value_impl(&mut self, value_type: crate::types::Type, kind: ValueKind) -> ValueRef {
        let vd = ValueDescriptor { value_type, kind };
        let index = self.value_arena.insert(vd);
        ValueRef { index }
    }
}

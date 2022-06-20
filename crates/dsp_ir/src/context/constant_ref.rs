use anyhow::Result;

use crate::constant::Constant;

use super::Context;

/// A reference to a constant in the constant table.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd, Copy, Clone)]
pub struct ConstantRef {
    index: generational_arena::Index,
}

/// Error yielded when resolving a constant fails.
#[derive(Debug, thiserror::Error)]
#[error("Resolution failed due to out of range indices in the constants table")]
pub struct ConstantResolutionFailed;

impl ConstantRef {
    pub(crate) fn resolve<'a>(&self, context: &'a Context) -> Result<&'a Constant> {
        Ok(context
            .constant_arena
            .get(self.index)
            .ok_or(ConstantResolutionFailed)?)
    }

    pub fn as_integral<'a>(&self, ctx: &'a Context) -> Result<Option<&'a [i64]>> {
        Ok(self.resolve(ctx)?.as_integral())
    }

    pub fn as_boolean<'a>(&self, ctx: &'a Context) -> Result<Option<&'a [bool]>> {
        Ok(self.resolve(ctx)?.as_boolean())
    }

    pub fn as_float<'a>(&self, ctx: &'a Context) -> Result<Option<&'a [rust_decimal::Decimal]>> {
        Ok(self.resolve(ctx)?.as_float())
    }
}

/// Methods on the context for dealing with constants.
impl Context {
    pub fn new_constant(&mut self, constant: crate::constant::Constant) -> ConstantRef {
        let index = self.constant_arena.insert(constant);
        ConstantRef { index }
    }
}

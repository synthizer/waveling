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
    pub fn resolve<'a>(&self, context: &'a Context) -> Result<&'a Constant> {
        Ok(context
            .constant_arena
            .get(self.index)
            .ok_or(ConstantResolutionFailed)?)
    }
}

/// Methods on the context for dealing with constants.
impl Context {
    pub fn new_constant(&mut self, constant: crate::constant::Constant) -> ConstantRef {
        let index = self.constant_arena.insert(constant);
        ConstantRef { index }
    }
}

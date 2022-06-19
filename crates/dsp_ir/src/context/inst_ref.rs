use anyhow::Result;

use super::*;

pub struct InstRef {
    index: generational_arena::Index,
}

#[derive(Debug, thiserror::Error)]
#[error("This instruction reference failed to resolve against the provided context")]
pub struct InstructionResolutionFailure;

impl InstRef {
    pub fn get_instruction<'a>(&self, ctx: &'a Context) -> Result<&'a crate::Instruction> {
        Ok(ctx
            .instruction_arena
            .get(self.index)
            .ok_or(InstructionResolutionFailure)?)
    }
}

impl Context {
    pub fn new_instruction(&mut self, inst: crate::Instruction) -> InstRef {
        let index = self.instruction_arena.insert(inst);
        InstRef { index }
    }
}

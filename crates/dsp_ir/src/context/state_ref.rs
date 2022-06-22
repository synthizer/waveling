use anyhow::Result;

use super::*;
use crate::*;

/// A reference to a state, an in-memory locationwhich is the same between program invocations unless set by the
/// program.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct StateRef {
    pub(super) index: generational_arena::Index,
}

pub struct StateDescriptor {
    state_type: crate::Type,
}

#[derive(Debug, thiserror::Error)]
#[error("Unable to resolve state against the context arena")]
pub struct StateResolutionFailed;

impl StateRef {
    fn resolve<'a>(&self, context: &'a Context) -> Result<&'a StateDescriptor> {
        Ok(context
            .state_arena
            .get(self.index)
            .ok_or(StateResolutionFailed)?)
    }

    pub fn get_type<'a>(&self, context: &'a Context) -> Result<&'a Type> {
        Ok(&self.resolve(context)?.state_type)
    }
}
impl StateDescriptor {
    pub fn get_type(&self) -> &crate::Type {
        &self.state_type
    }
}

impl Context {
    pub fn new_state(&mut self, state_type: crate::Type) -> StateRef {
        let index = self.state_arena.insert(StateDescriptor { state_type });
        StateRef { index }
    }
}

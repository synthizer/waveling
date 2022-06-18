use anyhow::Result;

use super::*;

/// A reference to a state, an in-memory locationwhich is the same between program invocations unless set by the
/// program.
#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct StateRef {
    index: generational_arena::Index,
}

pub struct StateDescriptor {
    pub state_type: crate::Type,
}
#[derive(Debug, thiserror::Error)]
#[error("Unable to resolve state against the context arena")]
pub struct StateResolutionFailed;

impl StateRef {
    pub fn resolve<'a>(&self, context: &'a Context) -> Result<&'a StateDescriptor> {
        Ok(context
            .state_arena
            .get(self.index)
            .ok_or(StateResolutionFailed)?)
    }
}

impl Context {
    pub fn new_state(&mut self, state_type: crate::Type) -> StateRef {
        let index = self.state_arena.insert(StateDescriptor { state_type });
        StateRef { index }
    }
}

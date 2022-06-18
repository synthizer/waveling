mod constant_ref;
mod state_ref;
mod value_ref;

pub use constant_ref::*;
pub use state_ref::*;
pub use value_ref::*;

use generational_arena::Arena;

use crate::constant::Constant;

#[derive(Default)]
pub struct Context {
    constant_arena: Arena<Constant>,
    value_arena: Arena<ValueDescriptor>,
    state_arena: Arena<StateDescriptor>,
}

impl Context {
    pub fn new() -> Context {
        Default::default()
    }
}

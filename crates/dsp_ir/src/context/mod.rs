mod constant_ref;
mod value_ref;

pub use constant_ref::*;
pub use value_ref::*;

use crate::constant::Constant;

#[derive(Default)]
pub struct Context {
    constant_table: Vec<Constant>,
    value_table: Vec<ValueDescriptor>,
}

impl Context {
    pub fn new() -> Context {
        Default::default()
    }
}

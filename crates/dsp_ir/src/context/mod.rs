mod constant_ref;
mod inst_ref;
mod state_ref;
mod value_ref;

pub use constant_ref::*;
pub use inst_ref::*;
pub use state_ref::*;
pub use value_ref::*;

use anyhow::Result;
use generational_arena::Arena;

use crate::constant::Constant;
use crate::Type;

pub struct Context {
    constant_arena: Arena<Constant>,
    value_arena: Arena<ValueDescriptor>,
    state_arena: Arena<StateDescriptor>,
    instruction_arena: Arena<crate::Instruction>,

    block_size: usize,
    inputs: Vec<crate::Type>,
    outputs: Vec<crate::Type>,
    properties: Vec<crate::Type>,

    /// the program's final execution order of instructions.
    ///
    /// Modified by various passes, then consumed by the backends.
    program: Vec<InstRef>,
}

impl Context {
    pub fn new(block_size: usize) -> Result<Context> {
        // Block size must be a power of 2, for now.
        if block_size == 0 {
            anyhow::bail!("Block size may not be 0");
        }

        if block_size & (block_size - 1) != 0 {
            anyhow::bail!("Block sizes must be a power of 2");
        }

        Ok(Context {
            constant_arena: Default::default(),
            value_arena: Default::default(),
            state_arena: Default::default(),
            instruction_arena: Default::default(),
            block_size,
            inputs: Default::default(),
            outputs: Default::default(),
            properties: Default::default(),
            program: Default::default(),
        })
    }

    /// Declare a new input and return the index.
    pub fn declare_input(&mut self, input_type: crate::Type) -> Result<usize> {
        if input_type.is_buffer() {
            anyhow::bail!("Inputs must be vectors");
        }
        self.inputs.push(input_type);
        Ok(self.inputs.len() - 1)
    }

    pub fn declare_output(&mut self, output_type: crate::Type) -> Result<usize> {
        if output_type.is_buffer() {
            anyhow::bail!("Output types may not be buffers");
        }

        self.outputs.push(output_type);
        Ok(self.outputs.len() - 1)
    }

    pub fn get_input_type(&self, index: usize) -> Option<&crate::Type> {
        self.inputs.get(index)
    }

    pub fn get_num_inputs(&self) -> usize {
        self.inputs.len()
    }

    pub fn iter_inputs(&self) -> impl Iterator<Item = (usize, &Type)> {
        self.inputs.iter().enumerate()
    }

    pub fn get_output_type(&self, index: usize) -> Option<&crate::Type> {
        self.outputs.get(index)
    }

    pub fn get_num_outputs(&self) -> usize {
        self.properties.len()
    }

    pub fn iter_outputs(&self) -> impl Iterator<Item = (usize, &Type)> {
        self.outputs.iter().enumerate()
    }

    /// Declare a property.
    ///
    /// The type argument is for future-proofing and must currently always be a scalar F64.
    pub fn new_property(&mut self, prop_type: crate::Type) -> Result<usize> {
        if prop_type != crate::Type::new_scalar(crate::types::Primitive::F64)? {
            anyhow::bail!("Currently, properties may only be scalar f64");
        }

        self.properties.push(prop_type);
        Ok(self.properties.len() - 1)
    }

    pub fn get_property_type(&self, index: usize) -> Option<&crate::Type> {
        self.properties.get(index)
    }

    pub fn get_num_properties(&self) -> usize {
        self.properties.len()
    }

    pub fn iter_properties(&self) -> impl Iterator<Item = (usize, &Type)> {
        self.properties.iter().enumerate()
    }

    pub fn get_block_size(&self) -> usize {
        self.block_size
    }
}

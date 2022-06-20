//! A reference implementation interpreter.
//!
//! This is horribly, horribly slow.  The point is that when fuzzing/testing other backends, running against this
//! interpreter can be used to compare outputs.
mod ops;

use std::collections::HashMap;

use anyhow::Result;
use smallvec::SmallVec;

use waveling_dsp_ir::types::Primitive;
use waveling_dsp_ir::*;

/// Every time we see a ValueRef as an output, we make a value and store it in a hashmap.
///
/// We treat all types as buffers and just store them as flat arrays.
#[derive(Debug)]
enum Value {
    I32(SmallVec<[i32; 32]>),
    I64(SmallVec<[i64; 16]>),
    F32(SmallVec<[f32; 32]>),
    F64(SmallVec<[f64; 16]>),
}

pub struct Interpreter {
    pub(crate) inputs: Vec<Vec<f32>>,
    pub(crate) outputs: Vec<Vec<f32>>,
    pub(crate) properties: Vec<f64>,

    /// Reset on every tick.
    pub(crate) values: HashMap<ValueRef, Value>,

    /// Stores state between ticks.
    pub(crate) state: HashMap<ValueRef, Value>,

    block_offset: u64,
    block_counter: u64,
}

impl Value {
    fn len(&self) -> usize {
        match self {
            Self::I32(x) => x.len(),
            Self::I64(x) => x.len(),
            Self::F32(x) => x.len(),
            Self::F64(x) => x.len(),
        }
    }
}

impl Interpreter {
    pub fn new(ctx: &Context) -> Result<Interpreter> {
        let mut interpreter = Interpreter {
            values: Default::default(),
            state: Default::default(),

            inputs: Default::default(),
            outputs: Default::default(),
            properties: Default::default(),
            block_offset: 0,
            block_counter: 0,
        };

        for (_, input) in ctx.iter_inputs() {
            if input.get_primitive() != Primitive::F32 {
                anyhow::bail!("Non-f32 input");
            }

            interpreter.inputs.push(vec![
                0.0;
                (input.get_vector_width() * ctx.get_block_size() as u64)
                    as usize
            ]);
        }

        for (_, output) in ctx.iter_outputs() {
            if output.get_primitive() != Primitive::F32 {
                anyhow::bail!("Non-f342 output");
            }

            interpreter.outputs.push(vec![
                0.0;
                (output.get_vector_width() * ctx.get_block_size() as u64)
                    as usize
            ]);
        }

        for (_, prop) in ctx.iter_properties() {
            if prop.get_primitive() != Primitive::F64 {
                anyhow::bail!("Non-f64 property");
            }

            interpreter.properties.push(0.0);
        }
        Ok(interpreter)
    }

    pub(crate) fn get_value_for_ref(&self, vref: ValueRef) -> Result<&Value> {
        self.values
            .get(&vref)
            .ok_or_else(|| anyhow::anyhow!("Value for ref not found"))
    }

    fn exec_one_instruction(&mut self, inst: &Instruction) -> Result<()> {
        use waveling_dsp_ir::Instruction as Inst;

        use ops::*;

        match inst {
            Inst::Add {
                output,
                left,
                right,
            } => add_vref(self, *output, *left, *right)?,
            Inst::Sub {
                output,
                left,
                right,
            } => sub_vref(self, *output, *left, *right)?,
            Inst::Mul {
                output,
                left,
                right,
            } => mul_vref(self, *output, *left, *right)?,
            Inst::Div {
                output,
                left,
                right,
            } => div_vref(self, *output, *left, *right)?,
            Inst::ModPositive {
                output,
                input,
                divisor,
            } => rem_vref(self, *output, *input, *divisor)?,
            Inst::Min {
                output,
                left,
                right,
            } => min_vref(self, *output, *left, *right)?,
            Inst::Max {
                output,
                left,
                right,
            } => max_vref(self, *output, *left, *right)?,
            Inst::Clamp {
                output,
                input,
                lower,
                upper,
            } => clamp_vref(self, *output, *input, *lower, *upper)?,
            Inst::Pow {
                output,
                base,
                exponent,
            } => pow_vref(self, *output, *base, *exponent)?,
            Inst::FastSin { output, input } => sin_vref(self, *output, *input)?,
            Inst::FastCos { output, input } => cos_vref(self, *output, *input)?,
            Inst::FastTan { input, output } => tan_vref(self, *output, *input)?,
            Inst::FastSinh { output, input } => sinh_vref(self, *output, *input)?,
            Inst::FastCosh { output, input } => cosh_vref(self, *output, *input)?,
            Inst::FastTanh { input, output } => tanh_vref(self, *output, *input)?,

            _ => anyhow::bail!("Unsupported instruction"),
        }

        Ok(())
    }

    /// Run one block.
    ///
    pub fn run_block(&mut self, ctx: &Context) -> Result<()> {
        for i in 0..ctx.get_block_size() {
            self.block_offset = i as u64;

            for inst in ctx.iter_instructions() {
                self.exec_one_instruction(inst)?;
            }
        }

        self.block_counter += 1;
        Ok(())
    }
}

//! A reference implementation interpreter.
//!
//! This is horribly, horribly slow.  The point is that when fuzzing/testing other backends, running against this
//! interpreter can be used to compare outputs.
mod ops;

use std::collections::HashMap;

use anyhow::Result;
use smallvec::{smallvec, SmallVec};

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

    /// These never change after initialization.
    pub(crate) constant_values: HashMap<ValueRef, Value>,

    /// Stores state between ticks.
    pub(crate) state: HashMap<StateRef, Value>,

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

    pub(crate) fn new_zero_from_ty(ty: &Type) -> Result<Value> {
        let length = (ty.get_vector_width() * ty.get_buffer_length()) as usize;

        let val = match ty.get_primitive() {
            Primitive::F32 => Value::F32(smallvec![0.0;length]),
            Primitive::F64 => Value::F64(smallvec![0.0;length]),
            Primitive::I32 => Value::I32(smallvec![0;length]),
            Primitive::I64 => Value::I64(smallvec![0;length]),
            Primitive::Bool => anyhow::bail!("Bool isn't supported yet"),
        };

        Ok(val)
    }
}

impl Interpreter {
    pub fn new(ctx: &Context) -> Result<Interpreter> {
        let mut interpreter = Interpreter {
            values: Default::default(),
            constant_values: Default::default(),
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

        for (sref, state) in ctx.iter_states() {
            interpreter
                .state
                .insert(sref, Value::new_zero_from_ty(state.get_type())?);
        }

        // We need to pre-resolve any constant values, because otherwise get_value_for_vref can't be const and that
        // makes everything much more difficult than it needs to be.
        for val in ctx.iter_values() {
            if let Some(c) = val.get_constant(ctx)? {
                use waveling_const::Constant::*;

                let prim = val.get_type(ctx)?.get_primitive();

                macro_rules! case {
                    ($var: ident, $ctx: ident, $x: ident, $target: ident) => {
                        match $x.resolve(&$ctx)? {
                            I32(ref y) => {
                                Value::$var(y.iter().copied().map(|x| x as $target).collect())
                            }
                            I64(ref y) => {
                                Value::$var(y.iter().copied().map(|i| i as $target).collect())
                            }
                            F32(ref y) => {
                                Value::$var(y.iter().copied().map(|i| i as $target).collect())
                            }
                            F64(ref y) => {
                                Value::$var(y.iter().copied().map(|i| i as $target).collect())
                            }
                            Bool(_) => anyhow::bail!("Bool is an unsupported constant type"),
                        }
                    };
                }

                let to_insert = match prim {
                    Primitive::F32 => {
                        case!(F32, ctx, c, f32)
                    }
                    Primitive::F64 => case!(F64, ctx, c, f64),
                    Primitive::I32 => case!(I32, ctx, c, i32),
                    Primitive::I64 => case!(I64, ctx, c, i64),
                    _ => anyhow::bail!("Unsupported type"),
                };

                interpreter.constant_values.insert(val, to_insert);
            }
        }

        Ok(interpreter)
    }

    pub(crate) fn get_value_for_ref(&self, vref: ValueRef) -> Result<&Value> {
        self.values
            .get(&vref)
            .or_else(|| self.constant_values.get(&vref))
            .ok_or_else(|| anyhow::anyhow!("Value for ref not found"))
    }

    fn exec_one_instruction(&mut self, ctx: &Context, inst: &Instruction) -> Result<()> {
        use waveling_dsp_ir::InstructionKind as Inst;

        use ops::*;

        match inst.get_kind() {
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
            Inst::ReadState {
                output,
                state,
                index,
            } => read_state_vref(self, ctx, *output, *state, *index, false)?,
            Inst::WriteState {
                input,
                state,
                index,
            } => write_state_vref(self, ctx, *input, *state, *index, false)?,
            Inst::ReadStateRelative {
                output,
                state,
                index,
            } => read_state_vref(self, ctx, *output, *state, *index, true)?,
            Inst::WriteStateRelative {
                input,
                state,
                index,
            } => write_state_vref(self, ctx, *input, *state, *index, true)?,
            Inst::ReadTimeSamples { output } => read_time_samples_vref(self, ctx, *output)?,
            Inst::ReadTimeSeconds { output } => read_time_seconds_vref(self, ctx, *output)?,
            Inst::ReadProperty {
                output,
                property_index: property,
            } => read_property_vref(self, *output, *property)?,
            Inst::ReadInput {
                output,
                input_index: input,
            } => read_input_vref(self, ctx, *output, *input)?,
            Inst::WriteOutput {
                output_index: input,
                index,
            } => write_output_vref(self, ctx, *input, *index)?,
            Inst::ToF32 { output, input } => to_f32_vref(self, *output, *input)?,
            Inst::ToF64 { output, input } => to_f64_vref(self, *output, *input)?,
        }

        Ok(())
    }

    /// Run one block.
    pub fn run_block(&mut self, ctx: &Context) -> Result<()> {
        for i in 0..ctx.get_block_size() {
            self.block_offset = i as u64;

            for inst in ctx.iter_instructions() {
                self.exec_one_instruction(ctx, inst)?;
            }

            // We clear the values on every tick because they are essentially named edges in the graph.
            self.values.clear();
        }

        self.block_counter += 1;
        Ok(())
    }

    fn get_time_in_samples(&self, ctx: &Context) -> u64 {
        self.block_counter * ctx.get_block_size() as u64 + self.block_offset
    }

    pub(crate) fn set_value(&mut self, vref: ValueRef, value: Value) -> Result<()> {
        if self.values.insert(vref, value).is_some() {
            anyhow::bail!("Attempt to double.set value");
        }

        Ok(())
    }

    pub fn write_input(&mut self, index: usize, data: &[f32]) -> Result<()> {
        let i_arr = self
            .inputs
            .get_mut(index)
            .ok_or_else(|| anyhow::anyhow!("Input {} not found", index))?;

        if i_arr.len() != data.len() {
            anyhow::bail!(
                "Input data is of the wrong size. Need exactly {} but got {}",
                i_arr.len(),
                data.len()
            );
        }

        (&mut i_arr[..]).copy_from_slice(data);
        Ok(())
    }

    pub fn read_output(&self, index: usize) -> Result<&[f32]> {
        let o = self
            .outputs
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("Invalid output index {}", index))?;
        Ok(o)
    }
}

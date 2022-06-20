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
    pub(crate) context: Context,

    /// Reset on every tick.
    pub(crate) values: HashMap<ValueRef, Value>,

    /// Stores state between ticks.
    pub(crate) state: HashMap<ValueRef, Value>,
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
    pub fn new(context: Context) -> Result<Interpreter> {
        let mut interpreter = Interpreter {
            values: Default::default(),
            state: Default::default(),
            context,
            inputs: Default::default(),
            outputs: Default::default(),
            properties: Default::default(),
        };

        for (_, input) in interpreter.context.iter_inputs() {
            if input.get_primitive() != Primitive::F32 {
                anyhow::bail!("Non-f32 input");
            }

            interpreter.inputs.push(vec![
                0.0;
                (input.get_vector_width() * interpreter.context.get_block_size() as u64)
                    as usize
            ]);
        }

        for (_, output) in interpreter.context.iter_outputs() {
            if output.get_primitive() != Primitive::F32 {
                anyhow::bail!("Non-f342 output");
            }

            interpreter.outputs.push(vec![
                0.0;
                (output.get_vector_width() * interpreter.context.get_block_size() as u64)
                    as usize
            ]);
        }

        for (_, prop) in interpreter.context.iter_properties() {
            if prop.get_primitive() != Primitive::F64 {
                anyhow::bail!("Non-f64 property");
            }

            interpreter.properties.push(0.0);
        }
        Ok(interpreter)
    }

    pub(crate) fn get_value_for_ref(&mut self, vref: ValueRef) -> Result<&Value> {
        let nval = if let Some(x) = vref.get_constant(&self.context)? {
            match vref.get_type(&self.context)?.get_primitive() {
                Primitive::F32 => {
                    let inner = x
                        .as_float(&self.context)?
                        .ok_or_else(|| {
                            anyhow::anyhow!("Cannot convert non-float constant to float")
                        })?
                        .iter()
                        .map(|i| Ok((*i).try_into()?))
                        .collect::<Result<_>>()?;
                    Value::F32(inner)
                }
                Primitive::F64 => {
                    let inner = x
                        .as_float(&self.context)?
                        .ok_or_else(|| {
                            anyhow::anyhow!("Cannot convert non-float constant to float")
                        })?
                        .iter()
                        .map(|i| Ok((*i).try_into()?))
                        .collect::<Result<_>>()?;
                    Value::F64(inner)
                }
                Primitive::I32 => {
                    let inner = x
                        .as_integral(&self.context)?
                        .ok_or_else(|| {
                            anyhow::anyhow!("Cannot convert non-float constant to float")
                        })?
                        .iter()
                        .map(|i| Ok((*i).try_into()?))
                        .collect::<Result<_>>()?;
                    Value::I32(inner)
                }
                Primitive::I64 => {
                    let inner = x
                        .as_integral(&self.context)?
                        .ok_or_else(|| {
                            anyhow::anyhow!("Cannot convert non-float constant to float")
                        })?
                        .iter()
                        .map(|i| Ok((*i).try_into()?))
                        .collect::<Result<_>>()?;
                    Value::I64(inner)
                }
                Primitive::Bool => anyhow::bail!("Bool not supported yet"),
            }
        } else {
            if let Some(x) = self.values.get(&vref) {
                return Ok(x);
            }

            anyhow::bail!(
                "Unable to resolve ValueRef because it was not already set and is not a constant"
            );
        };

        self.values.insert(vref, nval);
        Ok(self.values.get(&vref).expect("Just inserted"))
    }
}

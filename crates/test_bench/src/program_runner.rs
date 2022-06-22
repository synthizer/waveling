use anyhow::Result;

use waveling_dsp_ir::*;
use waveling_interpreter::Interpreter;

/// Helper function to run a program for one block.
///
/// This sets up the contexts, declares the inputs and outputs, runs the program, and returns a vec of vecs representing
/// the outputs in order.
pub(crate) fn run_program(
    sr: u64,
    block_size: usize,
    inputs: &[(Type, &[f32])],
    outputs: &[Type],
    builder: impl Fn(&mut Context) -> Result<()>,
) -> Result<Vec<Vec<f32>>> {
    let mut ctx = Context::new(sr, block_size)?;

    for i in inputs {
        ctx.declare_input(i.0)?;
    }

    for o in outputs {
        ctx.declare_output(*o)?;
    }

    builder(&mut ctx)?;

    let mut interpreter = Interpreter::new(&ctx)?;
    for (ind, i) in inputs.iter().enumerate() {
        interpreter.write_input(ind, i.1)?;
    }

    interpreter.run_block(&ctx)?;

    let mut outgoing = vec![];
    for i in 0..outputs.len() {
        outgoing.push(interpreter.read_output(i)?.to_vec());
    }

    Ok(outgoing)
}

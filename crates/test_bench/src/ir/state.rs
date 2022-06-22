use anyhow::Result;
use waveling_dsp_ir::constant::Constant;
use waveling_dsp_ir::inst_builder as ib;
use waveling_dsp_ir::*;

use crate::program_runner::run_program;

/// Test that we can use state to maintain a running sum.
#[test]
fn simple_accumulator() -> Result<()> {
    const BLOCK_SIZE: usize = 8;

    let got = run_program(
        44100,
        BLOCK_SIZE,
        &[],
        &[Type::new_vector(Primitive::F32, 1)?],
        |ctx| {
            let zero = ctx.new_value_const(
                Type::new_vector(Primitive::I32, 1)?,
                Constant::new_integral([0].into_iter())?,
            );
            let one = ctx.new_value_const(
                Type::new_vector(Primitive::I32, 1)?,
                Constant::new_integral([1].into_iter())?,
            );

            let state = ctx.new_state(Type::new_vector(Primitive::I32, 1)?);
            let read = ib::read_state(ctx, state, zero)?;
            let added = ib::add(ctx, read, one)?;
            ib::write_state(ctx, state, added, zero)?;
            let float = ib::to_f32(ctx, read)?;
            ib::write_output(ctx, float, 0)?;
            Ok(())
        },
    )?
    .pop()
    .unwrap();

    assert_eq!(got, vec![0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);

    Ok(())
}

/// A small 3-sample delay line with a constant fixed offset.
#[test]
fn delay_line() -> Result<()> {
    const BLOCK_SIZE: usize = 8;

    let input = (0..8usize)
        .into_iter()
        .map(|i| i as f32)
        .collect::<Vec<_>>();

    let got = run_program(
        44100,
        BLOCK_SIZE,
        &[(Type::new_vector(Primitive::F32, 1)?, &input[..])],
        &[Type::new_vector(Primitive::F32, 1)?],
        |ctx| {
            let zero = ctx.new_value_const(
                Type::new_vector(Primitive::I32, 1)?,
                Constant::new_integral([0].into_iter())?,
            );
            let neg_two = ctx.new_value_const(
                Type::new_vector(Primitive::I32, 1)?,
                Constant::new_integral([-2].into_iter())?,
            );

            let state = ctx.new_state(Type::new(Primitive::F32, 1, 3)?);
            let read = ib::read_state_relative(ctx, state, neg_two)?;
            let input = ib::read_input(ctx, 0)?;
            ib::write_state_relative(ctx, state, input, zero)?;
            ib::write_output(ctx, read, 0)?;
            Ok(())
        },
    )?
    .pop()
    .unwrap();

    assert_eq!(got, vec![0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);

    Ok(())
}

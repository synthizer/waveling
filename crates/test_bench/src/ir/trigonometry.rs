use anyhow::Result;
use paste::paste;
use waveling_dsp_ir::inst_builder as ib;
use waveling_dsp_ir::*;

use crate::program_runner::run_program;

/// Fill a single input with -2pi..2pi, and compare the output.
fn trig_test(
    trig_builder: impl Fn(&mut Context, ValueRef) -> Result<ValueRef>,
    trig_tester: impl Fn(f32) -> f32,
    width: u64,
) -> Result<()> {
    const BLOCK_SIZE: usize = 16;

    let total_vals = width * BLOCK_SIZE as u64;
    let idata = (0..total_vals)
        .into_iter()
        .map(|i| {
            use std::f32::consts::PI;

            (-2.0f32 * PI) + (i as f32 / (total_vals as f32 - 1.0f32)) * 4f32 * PI
        })
        .collect::<Vec<f32>>();

    let expected = idata.iter().map(|x| trig_tester(*x));

    let got = run_program(
        44100,
        BLOCK_SIZE as usize,
        &[(Type::new_vector(Primitive::F32, width)?, &idata[..])],
        &[Type::new_vector(Primitive::F32, width)?],
        |ctx| {
            let read = ib::read_input(ctx, 0)?;
            let trig = trig_builder(ctx, read)?;
            ib::write_output(ctx, trig, 0)?;
            Ok(())
        },
    )?
    .pop()
    .unwrap();

    crate::assert_float_arrays_same!(got, expected);

    Ok(())
}

macro_rules! decl_trig_test {
    ($name: ident, $builder:ident, $checker:ident) => {
        paste!(decl_trig_test!([<$name _1>], $builder, $checker, 1););
        paste!(decl_trig_test!([<$name _2>], $builder, $checker, 2););
    };

    ($name: ident, $builder:ident, $checker:ident, $width:expr) => {
        #[test]
        fn $name() -> Result<()> {
            trig_test(ib::$builder, |x| x.$checker(), $width)?;
            Ok(())
        }
    };
}

decl_trig_test!(sin, fast_sin, sin);
decl_trig_test!(cos, fast_cos, cos);
decl_trig_test!(tan, fast_tan, tan);
decl_trig_test!(sinh, fast_sinh, sinh);
decl_trig_test!(cosh, fast_cosh, cosh);
decl_trig_test!(tanh, fast_tanh, tanh);

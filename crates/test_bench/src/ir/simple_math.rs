//! Tests covering the basic mathematical operations, by spitting out programs that just apply the same operation on
//! both the Rust and the waveling side.
use anyhow::Result;
use paste::paste;
use waveling_dsp_ir::inst_builder as ib;
use waveling_dsp_ir::*;

use crate::program_runner::run_program;

/// Run a simple test against a binary operator, with a given input width.
///
/// The inputs are filled with consecutively increasing integers as floats, e.g. for mono [1,2,3,4] or stereo [(1, 2),
/// (3, 4)]... and so on.  The second input starts off where the first one left.
///
/// op_fact builds the program against the given context and expected_fn performs the binary operation we would expect
/// to see from Rust.
///
/// The arguments to op_fact are the context and the refs to the two inputs, and it must return the ref to the output.
///
/// Since the IR is capable of doing broadcasting, we want to abuse the fact that inputs can be different widths to test
/// that behavior.  To do so, we let the user specify the widths and work out what it would be on our side.  Either or
/// both of i1_width or i2_width should always be 1, or they should match.  Otherwise, the test is expected to fail.
fn test_binop_simple(
    op_fact: impl Fn(&mut Context, ValueRef, ValueRef) -> Result<ValueRef>,
    expected_fn: impl Fn(f32, f32) -> f32,
    i1_width: usize,
    i2_width: usize,
    o_width: usize,
) -> Result<()> {
    const BLOCK_SIZE: usize = 16;

    let i1_stops_at = BLOCK_SIZE * i1_width;
    let i2_stops_at = i1_stops_at + BLOCK_SIZE * i2_width;

    let d1 = (0..i1_stops_at)
        .into_iter()
        .map(|i| i as f32)
        .collect::<Vec<_>>();
    let d2 = (i1_stops_at..i2_stops_at)
        .into_iter()
        .map(|i| i as f32)
        .collect::<Vec<_>>();

    let mut got_outputs = run_program(
        44100,
        BLOCK_SIZE,
        &[
            (Type::new_vector(Primitive::F32, i1_width as u64)?, &d1[..]),
            (Type::new_vector(Primitive::F32, i2_width as u64)?, &d2[..]),
        ],
        &[Type::new_vector(Primitive::F32, o_width as u64)?],
        |ctx| {
            let in1_ref = ib::read_input(ctx, 0)?;
            let in2_ref = ib::read_input(ctx, 1)?;
            let res_ref = op_fact(ctx, in1_ref, in2_ref)?;
            ib::write_output(ctx, res_ref, 0)?;
            Ok(())
        },
    )?;

    let mut expected = vec![];
    for b in 0..BLOCK_SIZE {
        for ch in 0..i1_width.max(i2_width) {
            let i1_o = i1_width * b;
            let i2_o = i2_width * b;

            let i1_ind = i1_o + (ch % i1_width);
            let i2_ind = i2_o + (ch % i2_width);
            expected.push(expected_fn(d1[i1_ind], d2[i2_ind]));
        }
    }

    let got = got_outputs.pop().unwrap();
    crate::assert_float_arrays_same!(got, expected);

    Ok(())
}

macro_rules! binop {
    ($name: ident, $builder: ident, $checker: expr) => {
        paste!(binop!([<$name _1_1>], $builder, $checker, 1, 1, 1););
        paste!(binop!([<$name _1_2>], $builder, $checker, 1, 2, 2););
        paste!(binop!([<$name _2_1>], $builder, $checker, 2, 1, 2););
        paste!(binop!([<$name _2_2>], $builder, $checker, 2, 2, 2););
    };

    ($name: ident, $builder: ident, $checker: expr, $i1_w: expr, $i2_w: expr, $o_w: expr) => {
        #[test]
        fn $name() -> Result<()> {
            test_binop_simple(
                |ctx, left, right| waveling_dsp_ir::inst_builder::$builder(ctx, left, right),
                $checker,
                $i1_w,
                $i2_w,
                $o_w,
            )
        }
    };
}

binop!(add, add, |a, b| a + b);
binop!(sub, sub, |a, b| a - b);
binop!(mul, mul, |a, b| a * b);
binop!(div, div, |a, b| a / b);
binop!(pow, pow, |a, b| a.powf(b));

// We test modulus both ways since the first test here checks that it works if the input is less than the divisor, and
// the second one if the divisor is less than the input.
#[test]
fn mod_forward() -> Result<()> {
    test_binop_simple(ib::mod_positive, |a, b| a % b, 1, 1, 1)?;
    test_binop_simple(ib::mod_positive, |a, b| a % b, 1, 2, 2)?;
    test_binop_simple(ib::mod_positive, |a, b| a % b, 2, 1, 2)?;
    test_binop_simple(ib::mod_positive, |a, b| a % b, 2, 2, 2)?;
    Ok(())
}

#[test]
fn mod_rev() -> Result<()> {
    test_binop_simple(
        |ctx, a, b| ib::mod_positive(ctx, b, a),
        |a, b| b % a,
        1,
        1,
        1,
    )?;
    test_binop_simple(
        |ctx, a, b| ib::mod_positive(ctx, b, a),
        |a, b| b % a,
        1,
        2,
        2,
    )?;
    test_binop_simple(
        |ctx, a, b| ib::mod_positive(ctx, b, a),
        |a, b| b % a,
        2,
        1,
        2,
    )?;
    test_binop_simple(
        |ctx, a, b| ib::mod_positive(ctx, b, a),
        |a, b| b % a,
        2,
        2,
        2,
    )?;
    Ok(())
}

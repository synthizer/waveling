//! A set of methods to act as a convenient instruction builder which also verifies invariants.
//!
//! Programs will generally construct one of these, then use it to build up a list of operations by calling the methods
//! here in the order the instructions would need to run.  Note that in the model we use, there is no particular meaning
//! to order of operations as long as a value is ready when the next instruction that needs it begins to execute.  That
//! is, one way to think of this is like X86: a superscalar, out of order CPU.  It would be wrong to think of this as
//! establishing an order; the actual order is based on instruction input.
use anyhow::Result;

use crate::*;

fn must_be_same_primitive(ctx: &Context, i1: ValueRef, i2: ValueRef) -> Result<()> {
    if i1.get_type(ctx)?.get_primitive() != i2.get_type(ctx)?.get_primitive() {
        anyhow::bail!("Inputs must be of the same primitive type");
    }
    Ok(())
}

/// Validate that two inputs are either one vector and one constant, or two vectors of the same width.
fn validate_cv_pair_widths(ctx: &Context, i1: ValueRef, i2: ValueRef) -> Result<()> {
    let t1 = i1.get_type(ctx)?;
    let v1 = t1.get_vector_width();
    let t2 = i2.get_type(ctx)?;
    let v2 = t2.get_vector_width();

    if v1 != v2 && v1 != 1 && v2 != 1 {
        anyhow::bail!("Expected either a constant and a vector or two vectors of the same width; vector widths are {} and {}", v1, v2);
    }

    if t1.get_buffer_length() > 1 || t2.get_buffer_length() > 1 {
        anyhow::bail!(
            "This instruction does not work on buffers. Got lengths {} and {}",
            t1.get_buffer_length(),
            t2.get_buffer_length()
        );
    }

    Ok(())
}

/// Helper function to validate that two inputs are of the right type to act as inputs to an arithmetic instruction.
fn validate_arith_and_get_ty(ctx: &Context, left: ValueRef, right: ValueRef) -> Result<Type> {
    let ty1 = left.get_type(ctx)?;
    let ty2 = right.get_type(ctx)?;

    must_be_same_primitive(ctx, left, right)?;
    validate_cv_pair_widths(ctx, left, right)?;

    let prim = ty1.get_primitive();
    let out_with = ty1.get_vector_width().max(ty2.get_vector_width());

    Type::new_vector(prim, out_with)
}

macro_rules! arith {
    ($fn_name:ident, $variant: ident) => {
        arith!($fn_name, $variant, left, right);
    };

    ($fn_name: ident, $variant: ident, $left: ident, $right: ident) => {
        pub fn $fn_name(ctx: &mut Context, $left: ValueRef, $right: ValueRef) -> Result<ValueRef> {
            let ty = validate_arith_and_get_ty(ctx, $left, $right)?;
            let output = ctx.new_value(ty);
            ctx.new_instruction(InstructionKind::$variant {
                output,
                $left,
                $right,
            });
            Ok(output)
        }
    };
}

arith!(add, Add);
arith!(sub, Sub);
arith!(mul, Mul);
arith!(div, Div);
arith!(min, Min);
arith!(max, Max);
arith!(pow, Pow, base, exponent);
arith!(mod_positive, ModPositive, input, divisor);

pub fn clamp(
    ctx: &mut Context,
    input: ValueRef,
    lower: ValueRef,
    upper: ValueRef,
) -> Result<ValueRef> {
    // This validation is essentially two arithmetic instructions, a min and a max, and then just getting the type how
    // we would for that but accounting for the third place.
    must_be_same_primitive(ctx, input, lower)?;
    must_be_same_primitive(ctx, input, upper)?;
    validate_cv_pair_widths(ctx, input, lower)?;
    validate_cv_pair_widths(ctx, input, upper)?;

    let ty1 = input.get_type(ctx)?;
    let ty2 = lower.get_type(ctx)?;
    let ty3 = upper.get_type(ctx)?;

    let prim = ty1.get_primitive();
    let width = ty1
        .get_vector_width()
        .max(ty2.get_vector_width())
        .max(ty3.get_vector_width());
    let otype = Type::new_vector(prim, width)?;
    let output = ctx.new_value(otype);
    let inst = InstructionKind::Clamp {
        output,
        input,
        lower,
        upper,
    };
    ctx.new_instruction(inst);
    Ok(output)
}

// Only a couple of these for now, but almost certainly many more in future, and they're all the same.
macro_rules! conv {
    ($fn_name: ident, $variant: ident, $prim: ident) => {
        pub fn $fn_name(ctx: &mut Context, input: ValueRef) -> Result<ValueRef> {
            let ty = input.get_type(ctx)?;

            if ty.get_buffer_length() != 1 {
                anyhow::bail!("Conversion doesn't work on buffers");
            }

            let output = ctx.new_value(Type::new_vector(
                crate::types::Primitive::$prim,
                ty.get_vector_width(),
            )?);
            ctx.new_instruction(InstructionKind::$variant { output, input });
            Ok(output)
        }
    };
}

conv!(to_f32, ToF32, F32);
conv!(to_f64, ToF64, F64);

macro_rules! trig {
    ($fn_name: ident, $variant: ident) => {
        pub fn $fn_name(ctx: &mut Context, input: ValueRef) -> Result<ValueRef> {
            let ty = input.get_type(ctx)?;

            if ty.is_buffer() {
                anyhow::bail!("Trigonometry may not be performed on buffers");
            }

            if ty.get_primitive() != crate::types::Primitive::F32
                && ty.get_primitive() != crate::types::Primitive::F64
            {
                anyhow::bail!("Trig may only be performed on floating point types");
            }

            let output = ctx.new_value(ty);
            ctx.new_instruction(InstructionKind::$variant { output, input });
            Ok(output)
        }
    };
}

trig!(fast_sin, FastSin);
trig!(fast_cos, FastCos);
trig!(fast_tan, FastTan);
trig!(fast_sinh, FastSinh);
trig!(fast_cosh, FastCosh);
trig!(fast_tanh, FastTanh);

macro_rules! state {
    ($fn_name: ident, $variant: ident) => {
        pub fn $fn_name(ctx: &mut Context, state: StateRef, index: ValueRef) -> Result<ValueRef> {
            let ty = state.get_type(ctx)?;

            // The output type is this type, but minus the buffer part.
            let output_ty = crate::Type::new_vector(ty.get_primitive(), ty.get_vector_width())?;
            let output = ctx.new_value(output_ty);
            ctx.new_instruction(InstructionKind::$variant {
                output,
                state,
                index,
            });

            Ok(output)
        }
    };
}
state!(read_state, ReadState);
state!(read_state_relative, ReadStateRelative);

pub fn read_time_samples(ctx: &mut Context) -> Result<ValueRef> {
    let output = ctx.new_value(crate::Type::new_vector(crate::types::Primitive::I64, 1)?);
    ctx.new_instruction(InstructionKind::ReadTimeSamples { output });
    Ok(output)
}

pub fn read_time_seconds(ctx: &mut Context) -> Result<ValueRef> {
    let output = ctx.new_value(crate::Type::new_vector(crate::types::Primitive::F64, 1)?);
    ctx.new_instruction(InstructionKind::ReadTimeSeconds { output });
    Ok(output)
}

pub fn read_input(ctx: &mut Context, index: usize) -> Result<ValueRef> {
    let ty = *ctx
        .get_input_type(index)
        .ok_or_else(|| anyhow::anyhow!("Input index {} does not exist", index))?;
    let output = ctx.new_value(ty);
    ctx.new_instruction(InstructionKind::ReadInput {
        output,
        input_index: index,
    });
    Ok(output)
}

pub fn read_property(ctx: &mut Context, property: usize) -> Result<ValueRef> {
    let ty = *ctx
        .get_property_type(property)
        .ok_or_else(|| anyhow::anyhow!("Property index {} does not exist", property))?;
    let output = ctx.new_value(ty);
    ctx.new_instruction(InstructionKind::ReadProperty {
        output,
        property_index: property,
    });
    Ok(output)
}

pub fn write_output(ctx: &mut Context, input: ValueRef, index: usize) -> Result<()> {
    let ty = *ctx
        .get_output_type(index)
        .ok_or_else(|| anyhow::anyhow!("OUtput index {} not found", index))?;
    let ity = input.get_type(ctx)?;
    if ty != ity {
        anyhow::bail!(
            "Type mismatch between output and provided input: input={} output={}",
            ity,
            ty
        );
    }

    ctx.new_instruction(InstructionKind::WriteOutput {
        output_index: input,
        index,
    });
    Ok(())
}

#![allow(unused_parens)]
//! these are the operations of the interpreter implemented with macros so that the code duplication isn't a huge
//! problem.
use std::ops::{Add, Div, Mul, Rem, Sub};

use anyhow::{anyhow, Result};
use smallvec::{smallvec, SmallVec};
use waveling_dsp_ir::{Context, StateRef, ValueRef};

use crate::{Interpreter, Value};

macro_rules! op_impl {
    ($fn_name: ident, $trait: ident, $method: ident, $first_param: ident) => {
        op_impl!($fn_name, $trait, $method, $first_param,);
    };

    ($fn_name: ident, $trait: ident, $method: ident, $first_param: ident, $($rest: ident),*) => {
        pub(crate) fn $fn_name<T>(output: &mut [T], $first_param: &[T], $($rest: &[T]),*) -> Result<()>
        where
            T: $trait<Output = T> + Copy,
        {
            // We do this loop with a bit of unsafe for performance, and because it is very simple.
            unsafe {
                for i in 0..output.len() {
                        *output.get_unchecked_mut(i%output.len()) =
                            $first_param.get_unchecked(i%$first_param.len())
                            .$method($(*$rest.get_unchecked(i%$rest.len())),*);
                }
            }
            Ok(())
        }
    };
}

// Expand to a functionwhich dereferences value refs and calls the underlying impl.
//
// The macro rule here is weird because of nested expansions being required.
macro_rules! op_vref {
    ($fn_name: ident, $impl: ident, ($($sig_params: ident),*), $first_param: ident, $($variant: ident ($($param: ident),*)),+) => {
        pub(crate) fn $fn_name(
            interpreter: &mut Interpreter,
            output: ValueRef,
            $($sig_params: ValueRef),*
        ) -> Result<()> {
            $(let $sig_params = interpreter
                .get_value_for_ref($sig_params)?;
            )*

            let nlen = [$($sig_params.len()),*].iter().max().cloned().unwrap();
            let mut nv = None;
            $({
                use Value::$variant as Var;
                if let ($(Var($param)),*) = ($($param),*) {
                    let mut o = smallvec![Default::default(); nlen];
                    $impl(&mut o[..], $(&$param[..]),*)?;
                    nv = Some(Var(o));
                }
            })*

            if let Some(x) = nv {
                interpreter.set_value(output, x)?;
                return Ok(());
            }

            anyhow::bail!("Instruction operands must be of the same type");
        }
    };
}

macro_rules! binop {
    ($basename: ident, $trait: ident, $method: ident) => {
        binop!($basename, $trait, $method, I32, I64, F32, F64);
    };

    ($basename: ident, $trait: ident, $method: ident, $($variants:ident),*) => {
        paste::paste! {
            op_impl!([<$basename _ impl>], $trait, $method, left, right);
            op_vref!([<$basename _ vref>], [<$basename _ impl>], (left, right), left, $($variants(left, right)),*);
        }
    };
}

// Unary operator impl.
macro_rules! unop {
    ($basename: ident, $trait: ident, $method: ident) => {
        unop!($basename, $trait, $method, I32, I64, F32, F64);
    };

    ($basename: ident, $trait: ident, $method: ident, $($variants:ident),*) => {
        paste::paste! {
            op_impl!([<$basename _ impl>], $trait, $method, input);
            op_vref!([<$basename _ vref>], [<$basename _ impl>], (input), input, $($variants(input)),*);
        }
    };
}

// We can also treat min and max as binary operators, with a couple traits that look like std::ops enough for our
// macros.

pub(crate) trait Minable {
    type Output;

    fn do_min(&self, other: Self) -> Self::Output;
}

pub(crate) trait Maxable {
    type Output;

    fn do_max(&self, other: Self) -> Self::Output;
}

pub(crate) trait Power {
    type Output;

    fn do_power(&self, other: Self) -> Self::Output;
}

macro_rules! impl_binop_trait {
    ($trait: ident, $tmethod: ident, $cmethod: ident, $ty: ty) => {
        impl $trait for $ty {
            type Output = $ty;

            fn $tmethod(&self, other: $ty) -> Self::Output {
                (*self).$cmethod(other)
            }
        }
    };

    ($trait: ident, $tmethod: ident, $cmethod: ident, $head: ty, $($tys:ty),*) => {
        impl_binop_trait!($trait, $tmethod, $cmethod, $head);
        impl_binop_trait!($trait, $tmethod, $cmethod, $($tys),*);
    }
}

impl_binop_trait!(Minable, do_min, min, i32, i64, f32, f64);
impl_binop_trait!(Maxable, do_max, max, i32, i64, f32, f64);
impl_binop_trait!(Power, do_power, powf, f32, f64);

binop!(add, Add, add);
binop!(sub, Sub, sub);
binop!(mul, Mul, mul);
binop!(div, Div, div);
binop!(rem, Rem, rem);
binop!(min, Minable, do_min);
binop!(max, Maxable, do_max);
binop!(pow, Power, do_power, F32, F64);

macro_rules! trigtrait {
    ($name: ident, $do_method: ident, $method: ident) => {
        pub(crate) trait $name {
            type Output;

            fn $do_method(&self) -> Self::Output;
        }

        impl $name for f32 {
            type Output = f32;

            fn $do_method(&self) -> Self::Output {
                self.$method()
            }
        }

        impl $name for f64 {
            type Output = f64;

            fn $do_method(&self) -> Self::Output {
                self.$method()
            }
        }
    };
}

trigtrait!(TrigSin, do_sin, sin);
trigtrait!(TrigCos, do_cos, cos);
trigtrait!(TrigTan, do_tan, tan);
trigtrait!(TrigSinh, do_sinh, sinh);
trigtrait!(TrigCosh, do_cosh, cosh);
trigtrait!(TrigTanh, do_tanh, tanh);

unop!(sin, TrigSin, do_sin, F32, F64);
unop!(cos, TrigCos, do_cos, F32, F64);
unop!(tan, TrigTan, do_tan, F32, F64);
unop!(sinh, TrigSinh, do_sinh, F32, F64);
unop!(cosh, TrigCosh, do_cosh, F32, F64);
unop!(tanh, TrigTanh, do_tanh, F32, F64);

pub(crate) trait Clampable {
    type Output;

    fn do_clamp(&self, min: Self, max: Self) -> Self::Output;
}

macro_rules! clampable_impl {
    ($x:ty) => {
        impl Clampable for $x {
            type Output = $x;

            fn do_clamp(&self, min: $x, max: $x) -> Self::Output {
                (*self).clamp(min, max)
            }
        }
    };
}

clampable_impl!(i32);
clampable_impl!(i64);
clampable_impl!(f32);
clampable_impl!(f64);

op_impl!(clamp_impl, Clampable, do_clamp, value, lower, upper);
op_vref!(
    clamp_vref,
    clamp_impl,
    (value, lower, upper),
    value,
    I32(value, lower, upper),
    I64(value, lower, upper),
    F32(value, lower, upper),
    F64(value, lower, upper)
);

pub(crate) fn read_state_vref(
    interpreter: &mut Interpreter,
    ctx: &Context,
    output: ValueRef,
    state: StateRef,
    index: ValueRef,
    is_relative: bool,
) -> Result<()> {
    let sval = interpreter
        .state
        .get(&state)
        .ok_or_else(|| anyhow!("Attempt to read state before it was set"))?;

    let index = interpreter.get_value_for_ref(index)?;
    if index.len() != 1 {
        anyhow::bail!("Index must be a scalar");
    }

    let index = match index {
        Value::F32(_) | Value::F64(_) => anyhow::bail!("Index must be integral"),
        Value::I32(i) => *i.get(0).unwrap() as i64,
        Value::I64(i) => *i.get(0).unwrap() as i64,
    };

    let ty = state.get_type(ctx)?;
    let stride = ty.get_vector_width();
    let length = ty.get_buffer_length();

    if length == 1 && index != 0 {
        anyhow::bail!("Attempt to use a vector as a buffer");
    }

    let rel_off = if is_relative {
        interpreter.get_time_in_samples(ctx)
    } else {
        0
    };

    // The first rem_euclid makes sure we are only working with two positive numbers; the second, that the sum of the
    // index and the offset is in range.
    let will_read = ((index.rem_euclid(length.try_into()?) as u64 + rel_off).rem_euclid(length)
        * stride) as usize;
    let will_read_end = will_read + stride as usize;

    println!("{} {} {}", will_read, will_read_end, sval.len());
    if will_read as usize >= sval.len() || will_read_end > sval.len() {
        // This is an invariant internal to the interpreter because of the modulus.
        anyhow::bail!("Unable to read state because the index is out of range");
    }

    let read_val = match sval {
        Value::F32(x) => Value::F32(SmallVec::from_slice(&x[will_read..will_read_end])),
        Value::F64(x) => Value::F64(SmallVec::from_slice(&x[will_read..will_read_end])),
        Value::I32(x) => Value::I32(SmallVec::from_slice(&x[will_read..will_read_end])),
        Value::I64(x) => Value::I64(SmallVec::from_slice(&x[will_read..will_read_end])),
    };

    interpreter.set_value(output, read_val)?;
    Ok(())
}

pub(crate) fn write_state_vref(
    interpreter: &mut Interpreter,
    ctx: &Context,
    input: ValueRef,
    state: StateRef,
    index: ValueRef,
    is_relative: bool,
) -> Result<()> {
    let index = interpreter.get_value_for_ref(index)?;
    if index.len() != 1 {
        anyhow::bail!("Index must be a scalar");
    }

    let index = match index {
        Value::F32(_) | Value::F64(_) => anyhow::bail!("Index must be integral"),
        Value::I32(i) => *i.get(0).unwrap() as i64,
        Value::I64(i) => *i.get(0).unwrap() as i64,
    };

    let ty = state.get_type(ctx)?;
    let stride = ty.get_vector_width();
    let length = ty.get_buffer_length();

    if length == 1 && index != 0 {
        anyhow::bail!("Attempt to use a vector as a buffer");
    }

    let rel_off = if is_relative {
        interpreter.get_time_in_samples(ctx)
    } else {
        0
    };

    // The first rem_euclid brings the value into range and positive. The second brings our proposed read plus the
    // offset into range.
    let will_write = ((index.rem_euclid(length.try_into()?) as u64 + rel_off).rem_euclid(length)
        * stride) as usize;
    let will_write_end = will_write + stride as usize;

    let sval = interpreter
        .state
        .get_mut(&state)
        .ok_or_else(|| anyhow!("Attempt to write state before it was set"))?;

    let wval = interpreter
        .values
        .get(&input)
        .ok_or_else(|| anyhow!("Input value not set"))?;

    if will_write as usize >= sval.len() || will_write_end > sval.len() {
        // This is an invariant internal to the interpreter because of the modulus.
        anyhow::bail!("Unable to write state because the index is out of range");
    }

    match (sval, wval) {
        (Value::F32(d), Value::F32(s)) => {
            (&mut d[will_write..will_write_end]).copy_from_slice(&s[..])
        }
        (Value::F64(d), Value::F64(s)) => {
            (&mut d[will_write..will_write_end]).copy_from_slice(&s[..])
        }
        (Value::I32(d), Value::I32(s)) => {
            (&mut d[will_write..will_write_end]).copy_from_slice(&s[..])
        }
        (Value::I64(d), Value::I64(s)) => {
            (&mut d[will_write..will_write_end]).copy_from_slice(&s[..])
        }
        _ => anyhow::bail!("State must be the same type as the value to write"),
    }

    Ok(())
}

pub(crate) fn read_time_samples_vref(
    interpreter: &mut Interpreter,
    ctx: &Context,
    output: ValueRef,
) -> Result<()> {
    let val = Value::I64(smallvec![interpreter.get_time_in_samples(ctx) as i64]);

    interpreter.set_value(output, val)?;
    Ok(())
}

pub(crate) fn read_time_seconds_vref(
    interpreter: &mut Interpreter,
    ctx: &Context,
    output: ValueRef,
) -> Result<()> {
    let val = Value::F64(smallvec![
        interpreter.get_time_in_samples(ctx) as f64 / ctx.get_sr() as f64
    ]);

    interpreter.set_value(output, val)?;
    Ok(())
}

pub(crate) fn read_property_vref(
    interpreter: &mut Interpreter,
    output: ValueRef,
    property: usize,
) -> Result<()> {
    let val = *interpreter
        .properties
        .get(property)
        .ok_or_else(|| anyhow!("Property index {} out of range", property))?;

    interpreter.set_value(output, Value::F64(smallvec![val]))?;
    Ok(())
}

pub(crate) fn read_input_vref(
    interpreter: &mut Interpreter,
    ctx: &Context,
    output: ValueRef,
    input: usize,
) -> Result<()> {
    let ity = ctx
        .get_input_type(input)
        .ok_or_else(|| anyhow!("Input index {} out of range", input))?;

    let stride = ity.get_vector_width();

    let offset = interpreter.block_offset * stride;
    let end = offset + stride;

    let input_array = interpreter
        .inputs
        .get(input)
        .ok_or_else(|| anyhow!("Interpreter doesn't have an input to match {}", input))?;

    let val = Value::F32(SmallVec::from_slice(
        &input_array[offset as usize..end as usize],
    ));

    interpreter.set_value(output, val)?;
    Ok(())
}

pub(crate) fn write_output_vref(
    interpreter: &mut Interpreter,
    ctx: &Context,
    input: ValueRef,
    output: usize,
) -> Result<()> {
    let oty = ctx
        .get_output_type(output)
        .ok_or_else(|| anyhow!("Output index {} out of range", output))?;

    let stride = oty.get_vector_width();

    let o_arr = interpreter
        .outputs
        .get_mut(output)
        .ok_or_else(|| anyhow!("Output {} not found in interpreter", output))?;

    let val = interpreter
        .values
        .get(&input)
        .ok_or_else(|| anyhow!("Input vref not yet assigned a value"))?;

    match val {
        Value::F32(x) => {
            if x.len() != stride as usize {
                anyhow::bail!(
                    "Mismatch in instruction argument and output widths arg={} vs output={}",
                    x.len(),
                    stride
                );
            }

            let start = (interpreter.block_offset * stride) as usize;
            let end = start + stride as usize;
            (&mut o_arr[start..end]).copy_from_slice(&x[..]);
        }
        _ => anyhow::bail!("Only f32 may be twritten to outputs"),
    }

    Ok(())
}

pub(crate) fn to_f32_vref(
    interpreter: &mut Interpreter,
    output: ValueRef,
    input: ValueRef,
) -> Result<()> {
    let val_in = interpreter.get_value_for_ref(input)?;
    let val_out = match val_in {
        Value::F32(x) => Value::F32(x.clone()),
        Value::F64(x) => Value::F32(x.iter().cloned().map(|i| i as f32).collect()),
        Value::I32(x) => Value::F32(x.iter().cloned().map(|i| i as f32).collect()),
        Value::I64(x) => Value::F32(x.iter().cloned().map(|i| i as f32).collect()),
    };

    interpreter.set_value(output, val_out)?;

    Ok(())
}

pub(crate) fn to_f64_vref(
    interpreter: &mut Interpreter,
    output: ValueRef,
    input: ValueRef,
) -> Result<()> {
    let val_in = interpreter.get_value_for_ref(input)?;
    let val_out = match val_in {
        Value::F32(x) => Value::F64(x.iter().cloned().map(|i| i as f64).collect()),
        Value::F64(x) => Value::F64(x.clone()),
        Value::I32(x) => Value::F64(x.iter().cloned().map(|i| i as f64).collect()),
        Value::I64(x) => Value::F64(x.iter().cloned().map(|i| i as f64).collect()),
    };

    interpreter.set_value(output, val_out)?;

    Ok(())
}

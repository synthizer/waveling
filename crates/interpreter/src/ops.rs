#![allow(unused_parens)]
//! these are the operations of the interpreter implemented with macros so that the code duplication isn't a huge
//! problem.
use std::ops::{Add, Div, Mul, Rem, Sub};

use anyhow::Result;
use smallvec::smallvec;
use waveling_dsp_ir::ValueRef;

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

            $({
                if $sig_params.len() != $first_param.len() {
                    anyhow::bail!("All operands must be of the same width");
                }
            })*

            let mut nv = None;
            $({
                use Value::$variant as Var;
                if let ($(Var($param)),*) = ($($param),*) {
                    let mut o = smallvec![Default::default(); $first_param.len()];
                    $impl(&mut o[..], $(&$param[..]),*)?;
                    nv = Some(Var(o));
                }
            })*


            if let Some(x) = nv {
                if interpreter.values.insert(output, x).is_some() {
                    anyhow::bail!("Attempt to double-set a value");
                }
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

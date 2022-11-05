use std::fmt::Display;

use crate::{PrimitiveType, VectorDescriptor};

/// A vector constant.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Constant {
    Bool(Vec<bool>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
}

/// Reasons why we can't carry out an operation between two constants.
///
/// This comes up when constant folding and when running the interpreter.
#[derive(Debug, thiserror::Error)]
pub enum ConstantFoldingError {
    #[error("Cannot apply constant operations to a constant of width 0")]
    ZeroWidthConstant,

    #[error("Attempt to fold constants of incompatible types")]
    IncompatibleTypes,

    #[error("Attempt to use a constnat of an unsuported type for this operation")]
    UnsupportedType,

    #[error("Constant widths are not the same, and neither can be broadcast")]
    IncompatibleWidths,
}

impl Constant {
    pub fn primitive_type(&self) -> PrimitiveType {
        match self {
            Self::Bool(_) => PrimitiveType::Bool,
            Self::I64(_) => PrimitiveType::I64,
            Self::F32(_) => PrimitiveType::F32,
            Self::F64(_) => PrimitiveType::F64,
        }
    }

    pub fn width(&self) -> u64 {
        let w = match self {
            Self::Bool(v) => v.len(),
            Self::I64(v) => v.len(),
            Self::F32(v) => v.len(),
            Self::F64(v) => v.len(),
        };

        w as u64
    }

    pub fn vector_descriptor(&self) -> VectorDescriptor {
        VectorDescriptor {
            primitive: self.primitive_type(),
            width: self.width(),
        }
    }
}

impl Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use itertools::Itertools;

        let (ty, inner) = match self {
            Constant::Bool(x) => ("bool", x.iter().join(", ")),
            Constant::F32(x) => ("f32", x.iter().join(", ")),
            Constant::F64(x) => ("f64", x.iter().join(", ")),
            Constant::I64(x) => ("i64", x.iter().join(", ")),
        };

        write!(f, "{}[{}]", ty, inner)
    }
}

fn do_binop(
    left: &Constant,
    right: &Constant,
    bool_case: Option<&mut dyn FnMut(bool, bool) -> bool>,
    i64_case: Option<&mut dyn FnMut(i64, i64) -> i64>,
    f32_case: Option<&mut dyn FnMut(f32, f32) -> f32>,
    f64_case: Option<&mut dyn FnMut(f64, f64) -> f64>,
) -> Result<Constant, ConstantFoldingError> {
    if left.width() == 0 || right.width() == 0 {
        return Err(ConstantFoldingError::ZeroWidthConstant);
    }

    if left.width() != right.width() && left.width() != 1 && right.width() != 1 {
        return Err(ConstantFoldingError::IncompatibleWidths);
    }

    let total_len = left.width().max(right.width());
    macro_rules! arm {
        ($output_variant: ident, $l: ident, $r: ident, $case_var: ident) => {{
            let case_fn = $case_var.ok_or(ConstantFoldingError::UnsupportedType)?;
            Ok(Constant::$output_variant(
                (0..total_len)
                    .into_iter()
                    .map(|i| case_fn($l[(i % total_len) as usize], $r[(i % total_len) as usize]))
                    .collect(),
            ))
        }};
    }

    use Constant::*;

    match (left, right) {
        (Bool(l), Bool(r)) => arm!(Bool, l, r, bool_case),
        (I64(l), I64(r)) => arm!(I64, l, r, i64_case),
        (F32(l), F32(r)) => arm!(F32, l, r, f32_case),
        (F64(l), F64(r)) => arm!(F64, l, r, f64_case),
        (_, _) => Err(ConstantFoldingError::IncompatibleTypes),
    }
}

/// Punch out operations which work on i64/f32/f64.
macro_rules! numeric_binop {
    ($op_name: ident, $trait: ident) => {
        paste::paste! {
            pub fn [<fold_ $op_name>](&self, other: &Constant) -> Result<Constant, ConstantFoldingError> {
                use std::ops::$trait;

                do_binop(
                    self,
                    other,
                    None,
                    Some(&mut |a: i64, b: i64| a.$op_name(b)),
                    Some(&mut |a: f32, b: f32| a.$op_name(b)),
                    Some(&mut |a: f64, b: f64| a.$op_name(b))
                )
            }
        }
    }
}

/// # Mathematical operations between constants.
///
/// These are used for constant folding, and also for the interpreters.
impl Constant {
    numeric_binop!(add, Add);
    numeric_binop!(sub, Sub);
    numeric_binop!(mul, Mul);
    numeric_binop!(div, Div);
    numeric_binop!(rem, Rem);

    /// Negate this constant.
    pub fn fold_neg(&self) -> Result<Constant, ConstantFoldingError> {
        // binop also does unary operations, if we simply let both sides be the same.
        do_binop(
            self,
            self,
            None,
            Some(&mut |a, _b| -a),
            Some(&mut |a, _b| -a),
            Some(&mut |a, _b| -a),
        )
    }
}

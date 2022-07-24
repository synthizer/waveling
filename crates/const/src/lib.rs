use smallvec::SmallVec;

use waveling_diagnostics::*;

pub type Result<T> = std::result::Result<T, CompilationError>;

/// A constant, an i32, i64, f32, f64, or bool.
pub enum Constant {
    I32(SmallVec<[i32; 8]>),
    I64(SmallVec<[i64; 4]>),
    F32(SmallVec<[f32; 8]>),
    F64(SmallVec<[f64; 4]>),
    Bool(SmallVec<[bool; 32]>),
}

/// Given two constants of the same type and dimension or two constants where one is scalar, apply the requisite closure
/// and return a new constant of the same type.
///
/// If one of the constants is scalar, broadcast as if it were the wider constant's width all filled with the scalar
/// value.
///
/// Used primarily as part of the macros to generate all the try_whatever functions for binary ops.
#[allow(clippy::too_many_arguments)]
fn broadcasting_op(
    span: Option<Span>,
    left: &Constant,
    right: &Constant,
    i32_case: impl Fn(Option<Span>, i32, i32) -> Result<i32>,
    i64_case: impl Fn(Option<Span>, i64, i64) -> Result<i64>,
    f32_case: impl Fn(Option<Span>, f32, f32) -> Result<f32>,
    f64_case: impl Fn(Option<Span>, f64, f64) -> Result<f64>,
    bool_case: impl Fn(Option<Span>, bool, bool) -> Result<bool>,
) -> Result<Constant> {
    if left.get_width() == 0 || right.get_width() == 0 {
        return Err(CompilationError::new(
            span,
            "Mathematical operations with a constant of zero width are not possible",
        ));
    }

    if left.get_width() != right.get_width() && left.get_width() != 1 && right.get_width() != 1 {
        return Err(CompilationError::new(
            span,
            "Cannot multiply constants of different dimensions unless one of them is scalar",
        ));
    }

    macro_rules! case {
        ($var: ident,  $l: expr, $r: expr, $case: expr) => {{
            let new_vec = (0..$l.len().max($r.len()))
                .into_iter()
                .map(|i| $case(span, $l[i % $l.len()], $r[i % $r.len()]))
                .collect::<Result<_>>()?;
            Ok(Constant::$var(new_vec))
        }};
    }

    match (left, right) {
        (Constant::I32(ref l), Constant::I32(ref r)) => case!(I32, l, r, i32_case),
        (Constant::I64(ref l), Constant::I64(ref r)) => case!(I64, l, r, i64_case),
        (Constant::F32(ref l), Constant::F32(ref r)) => case!(F32, l, r, f32_case),
        (Constant::F64(ref l), Constant::F64(ref r)) => case!(F64, l, r, f64_case),
        (Constant::Bool(ref l), Constant::Bool(ref r)) => case!(Bool, l, r, bool_case),
        _ => Err(CompilationError::new(
            span,
            "This operation is not supported on mixed types",
        )),
    }
}

fn type_unsupported<T>(span: Option<Span>, type_str: &str) -> Result<T> {
    Err(CompilationError::new(
        span,
        &format!("Operation not supported on {}", type_str),
    ))
}

macro_rules! try_binop {
    ($name: ident, $closure: expr) => {
        pub fn $name(&self, span: Option<Span>, right: &Constant) -> Result<Constant> {
            broadcasting_op(
                span,
                self,
                right,
                $closure,
                $closure,
                $closure,
                $closure,
                |span, _a, _b| type_unsupported(span, "bool"),
            )
        }
    };
}

impl Constant {
    pub fn get_width(&self) -> usize {
        match *self {
            Constant::I32(ref x) => x.len(),
            Constant::I64(ref x) => x.len(),
            Constant::F32(ref x) => x.len(),
            Constant::F64(ref x) => x.len(),
            Constant::Bool(ref x) => x.len(),
        }
    }

    try_binop!(try_add, |_span, a, b| Ok(a + b));
    try_binop!(try_sub, |_span, a, b| Ok(a - b));
    try_binop!(try_mul, |_span, a, b| Ok(a * b));
    try_binop!(try_div, |_span, a, b| Ok(a / b));
    try_binop!(try_min, |_span, a, b| Ok(a.min(b)));
    try_binop!(try_max, |_span, a, b| Ok(a.max(b)));

    /// Works on (f32, f32) or (f64, f64).
    pub fn try_pow(&self, span: Option<Span>, right: &Constant) -> Result<Constant> {
        broadcasting_op(
            span,
            self,
            right,
            |span, _a, _b| type_unsupported(span, "i32"),
            |span, _a, _b| type_unsupported(span, "i64"),
            |_span, a, b| Ok(a.powf(b)),
            |_span, a, b| Ok(a.powf(b)),
            |span, _a, _b| type_unsupported(span, "bool"),
        )
    }

    pub fn try_clamp(
        &self,
        span: Option<Span>,
        min: &Constant,
        max: &Constant,
    ) -> Result<Constant> {
        self.try_min(span, max)?.try_max(span, min)
    }
}

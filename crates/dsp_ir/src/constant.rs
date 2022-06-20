use std::convert::TryInto;

use anyhow::Result;
use rust_decimal::Decimal;
use smallvec::SmallVec;

/// A constant.
///
/// Constants are "untyped", and instead get a species: boolean, integral, or float. The job of the backends is to
/// convert these as needed.  We store float as a decimal value in order to facilitate constant folding in the IR; many
/// dsp algorithms are sensitive to f32 inaccuracy or f64 inaccuracy if pushed far enough, so we fold in the higher
/// precision for long enough to collapse what can be collapsed.
///
/// Note that while this setup technically allows for zero-width constants, that's ill-formed.
pub struct Constant {
    inner: ConstantInner,
}

enum ConstantInner {
    Boolean(SmallVec<[bool; 64]>),
    Integral(SmallVec<[i64; 8]>),
    Float(SmallVec<[Decimal; 4]>),
}

#[derive(thiserror::Error)]
pub enum ConstantConstructionError<T> {
    #[error("Constants must not be zero width")]
    ZeroWidth,

    #[error("Got error converting: {0}")]
    Conversion(#[from] T),
}

impl Constant {
    pub fn new_integral<T: TryInto<i64>>(
        &self,
        iter: impl Iterator<Item = T>,
    ) -> Result<Constant, ConstantConstructionError<T::Error>>
    where
        T::Error: std::error::Error,
    {
        let inner = iter
            .map(|x| x.try_into())
            .collect::<Result<SmallVec<_>, T::Error>>()?;

        if inner.is_empty() {
            return Err(ConstantConstructionError::ZeroWidth);
        }

        Ok(Constant {
            inner: ConstantInner::Integral(inner),
        })
    }

    pub fn new_float<T: TryInto<Decimal>>(
        &self,
        iter: impl Iterator<Item = T>,
    ) -> Result<Constant, ConstantConstructionError<T::Error>>
    where
        T::Error: std::error::Error,
    {
        let inner = iter
            .map(|x| x.try_into())
            .collect::<Result<SmallVec<_>, T::Error>>()?;

        if inner.is_empty() {
            return Err(ConstantConstructionError::ZeroWidth);
        }

        Ok(Constant {
            inner: ConstantInner::Float(inner),
        })
    }

    pub fn new_boolean<T: TryInto<bool>>(
        &self,
        iter: impl Iterator<Item = T>,
    ) -> Result<Constant, ConstantConstructionError<T::Error>>
    where
        T: std::error::Error,
    {
        let inner = iter
            .map(|x| x.try_into())
            .collect::<Result<SmallVec<_>, T::Error>>()?;

        if inner.is_empty() {
            return Err(ConstantConstructionError::ZeroWidth);
        }

        Ok(Constant {
            inner: ConstantInner::Boolean(inner),
        })
    }

    pub fn as_integral(&self) -> Option<&[i64]> {
        if let ConstantInner::Integral(ref x) = self.inner {
            Some(&x[..])
        } else {
            None
        }
    }

    pub fn as_boolean(&self) -> Option<&[bool]> {
        if let ConstantInner::Boolean(ref x) = self.inner {
            Some(&x[..])
        } else {
            None
        }
    }

    pub fn as_float(&self) -> Option<&[Decimal]> {
        if let ConstantInner::Float(ref x) = self.inner {
            Some(&x[..])
        } else {
            None
        }
    }

    pub fn is_boolean(&self) -> bool {
        self.as_boolean().is_some()
    }

    pub fn is_integral(&self) -> bool {
        self.as_integral().is_some()
    }

    pub fn is_float(&self) -> bool {
        self.as_float().is_some()
    }

    pub fn width(&self) -> usize {
        match self.inner {
            ConstantInner::Boolean(ref x) => x.len(),
            ConstantInner::Float(ref x) => x.len(),
            ConstantInner::Integral(ref x) => x.len(),
        }
    }
}

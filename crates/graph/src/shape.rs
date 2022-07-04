#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, strum::Display)]
pub(crate) enum ShapeDataType {
    I32,
    I64,
    F32,
    F64,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Shape {
    pub(crate) data_type: ShapeDataType,
    pub(crate) width: usize,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ShapeMergeError {
    #[error("Cannot merge {0} signals withb {1} signals")]
    IncompatibleTypes(ShapeDataType, ShapeDataType),

    #[error("Cannot merge signals with incompatible widths {0} and {1}")]
    IncompatibleWidths(usize, usize),
}

impl Shape {
    pub(crate) fn new(data_type: ShapeDataType, width: usize) -> Shape {
        Shape { data_type, width }
    }

    /// Merge this shape with another one, returnning a new shape or an error.
    ///
    /// The error describes why the shapes couldn't merge.
    ///
    /// two shapes are compatible if they are of the same type and width, or if they are of the same type and one is of
    /// width 1.
    pub fn merge(&self, other: &Shape) -> Result<Shape, ShapeMergeError> {
        if self.data_type != other.data_type {
            return Err(ShapeMergeError::IncompatibleTypes(
                self.data_type,
                other.data_type,
            ));
        }

        if self.width == other.width || self.width == 1 || other.width == 1 {
            return Ok(Shape::new(self.data_type, self.width.max(other.width)));
        }

        Err(ShapeMergeError::IncompatibleWidths(self.width, other.width))
    }

    pub fn is_scalar(&self) -> bool {
        self.width == 1
    }
}

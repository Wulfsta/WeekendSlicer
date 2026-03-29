use std::fmt;
use strum::Display;

#[derive(Debug, Clone)]
pub struct Mesh2FrepError {
    pub kind: Mesh2FrepErrorKind,
    message: String,
}

impl Mesh2FrepError {
    pub fn create(m2fek: Mesh2FrepErrorKind) -> Mesh2FrepError {
        match m2fek {
            Mesh2FrepErrorKind::DegenerateMatrix => Mesh2FrepError {
                kind: Mesh2FrepErrorKind::DegenerateMatrix,
                message: "A Matrix was not invertable.".to_string(),
            },
            Mesh2FrepErrorKind::BadTetrahedrons => Mesh2FrepError {
                kind: Mesh2FrepErrorKind::BadTetrahedrons,
                message: "Expected linear tet4 cells.".to_string(),
            },
            _ => Mesh2FrepError {
                kind: Mesh2FrepErrorKind::UnenumeratedError,
                message: "An error originated in the transformation pipeline. This error code should be changed to a more specific one.".to_string(),
            },
        }
    }
}

impl std::error::Error for Mesh2FrepError {}

impl From<tritet::StrError> for Mesh2FrepError {
    fn from(e: tritet::StrError) -> Self {
        Mesh2FrepError {
            kind: Mesh2FrepErrorKind::TetGenError,
            message: e.to_string(),
        }
    }
}

impl fmt::Display for Mesh2FrepError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

#[derive(Debug, Clone, Display)]
pub enum Mesh2FrepErrorKind {
    DegenerateMatrix,
    TetGenError,
    BadTetrahedrons,
    UnenumeratedError,
}

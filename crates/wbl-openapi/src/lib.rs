//! OpenAPI for Whimbrel (OPENAPI.md, D-H14): Swagger 2.0 and OpenAPI 3.0 to 3.2, JSON or YAML,
//! read into one model (3.2 as the superset) and turned into collections.

mod build;
pub mod generate;
pub mod install;
pub mod model;
mod operations;
pub mod read;
mod refs;
pub mod stored;
mod swagger2;
pub mod synth;

pub use build::{build, detect};
pub use generate::{Generated, GeneratedRequest, generate};
pub use install::{Installed, Source, install};
pub use model::*;
pub use read::{Documents, Loaded, Loader, load_all, parse};
pub use stored::{StoredSpec, source_file, specs};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OpenApiError {
    #[error("The document cannot be read: {0}")]
    Syntax(String),
    #[error("This is not an OpenAPI document: {0}")]
    NotOpenApi(String),
    #[error("{0}")]
    Load(String),
}

impl OpenApiError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Syntax(_) => "openapi_syntax",
            Self::NotOpenApi(_) => "openapi_unknown",
            Self::Load(_) => "openapi_load",
        }
    }
}

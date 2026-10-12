//! Current browser save contract. Capture and reconstruction use stable source identities.
pub mod identity;
pub mod schema;
mod validate;

pub use identity::*;
pub use schema::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveError {
    CorruptData(String),
    IncompatibleSources(String),
    InvalidLinks(String),
    ReconstructionFailure(String),
    MissingAssets(String),
}
impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (kind, detail) = match self {
            Self::CorruptData(s) => ("Corrupt save", s),
            Self::IncompatibleSources(s) => ("Incompatible save", s),
            Self::InvalidLinks(s) => ("Invalid save links", s),
            Self::ReconstructionFailure(s) => ("Reconstruction failed", s),
            Self::MissingAssets(s) => ("Missing source asset", s),
        };
        write!(f, "{kind}: {detail}")
    }
}
impl std::error::Error for SaveError {}

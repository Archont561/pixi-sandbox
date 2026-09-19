use thiserror::Error;

#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("error[E-USAGE]: {0}\n = try: pixi-sandbox --help")]
    Usage(String),

    #[error("error[E-INTEGRITY]: {0}")]
    Integrity(String),

    #[error("error[E-UNAVAILABLE]: {0}")]
    Unavailable(String),

    #[error("error[E-NOT-DETECTED]: {0}")]
    NotDetected(String),

    #[error("error[E-VENDOR-INCOMPLETE]: {0}")]
    VendorIncomplete(String),

    #[error("error[E-RECONSTRUCT]: {0}")]
    Reconstruct(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl SandboxError {
    pub fn code(&self) -> u8 {
        match self {
            SandboxError::Usage(_) => 1,
            SandboxError::Integrity(_) => 4,
            SandboxError::Unavailable(_) => 5,
            SandboxError::NotDetected(_) => 7,
            SandboxError::VendorIncomplete(_) => 8,
            SandboxError::Reconstruct(_) => 8,
            SandboxError::Other(_) => 1,
        }
    }
}

pub type Result<T> = std::result::Result<T, SandboxError>;

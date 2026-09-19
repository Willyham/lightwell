//! Stable internal error categories shared by GUI and development drivers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Startup,
    UnsupportedInput,
    UnsupportedColor,
    UnsupportedProfile,
    FileAccess,
    Decode,
    ResourceLimit,
    Render,
    Diagnostics,
    Internal,
}
impl ErrorKind {
    pub fn code(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::UnsupportedInput => "unsupported-input",
            Self::UnsupportedColor => "unsupported-color",
            Self::UnsupportedProfile => "unsupported-profile",
            Self::FileAccess => "read-error",
            Self::Decode => "invalid-input",
            Self::ResourceLimit => "resource-limit",
            Self::Render => "render",
            Self::Diagnostics => "diagnostics",
            Self::Internal => "internal",
        }
    }
}
#[derive(Debug, Clone)]
pub struct Error {
    pub kind: ErrorKind,
    pub detail: String,
}
impl Error {
    pub fn new(kind: ErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
    // Normalize decoder-adapter diagnostics at the shared application boundary.
    pub(crate) fn decoder(detail: String) -> Self {
        let kind = match detail.split(':').next().unwrap_or("") {
            "read-error" => ErrorKind::FileAccess,
            "resource-limit" => ErrorKind::ResourceLimit,
            "unsupported-input" => ErrorKind::UnsupportedInput,
            "unsupported-color" => ErrorKind::UnsupportedColor,
            "unsupported-profile" => ErrorKind::UnsupportedProfile,
            "invalid-input" => ErrorKind::Decode,
            _ => ErrorKind::Internal,
        };
        Self::new(kind, detail)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind.code(), self.detail)
    }
}
impl std::error::Error for Error {}

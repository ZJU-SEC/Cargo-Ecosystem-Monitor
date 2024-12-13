#[derive(Debug)]
pub enum AuditError {
    /// For unexpected errors.
    InnerError(String),
    /// Fix failure errors.
    FunctionError(String),
}

impl AuditError {
    /// Is it an inner error or just fixing failure.
    pub fn is_inner(&self) -> bool {
        match self {
            Self::InnerError(_) => true,
            _ => false,
        }
    }

    pub fn into_msg(self) -> String {
        match self {
            Self::InnerError(s) => s,
            Self::FunctionError(s) => s,
        }
    }
}

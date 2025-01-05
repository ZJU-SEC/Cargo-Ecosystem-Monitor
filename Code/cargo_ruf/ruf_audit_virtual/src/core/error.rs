use cargo_lock::dependency::graph::NodeIndex;

#[derive(Debug)]
pub enum AuditError {
    /// For unexpected errors.
    InnerError(String),
    /// Fix failure errors, and record which dep cause it.
    FunctionError(Option<String>, Option<NodeIndex>),
}

impl AuditError {
    /// Is it an inner error or just fixing failure.
    pub fn is_inner(&self) -> bool {
        match self {
            Self::InnerError(_) => true,
            _ => false,
        }
    }
}

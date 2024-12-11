#[derive(Debug)]
pub enum AuditError {
    /// For unexpected errors.
    InnerError(String),
    /// Fix failure errors.
    FunctionError(String)
}
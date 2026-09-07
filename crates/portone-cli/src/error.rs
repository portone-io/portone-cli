#[derive(Debug)]
pub enum CliError {
    Silent,
    Flag(String),
    Other(anyhow::Error),
}

impl From<anyhow::Error> for CliError {
    fn from(err: anyhow::Error) -> Self {
        CliError::Other(err)
    }
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        CliError::Other(err.into())
    }
}

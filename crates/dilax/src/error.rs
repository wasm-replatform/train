use thiserror::Error;

//  TODO: refer to r9k errors

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to parse timestamp: {0}")]
    Timestamp(String),

    #[error("failed to parse token from Dilax payload: {0}")]
    Token(String),

    #[error("unable to serialize state: {0}")]
    State(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

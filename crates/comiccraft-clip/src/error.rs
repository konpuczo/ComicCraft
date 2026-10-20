use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClipError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("not a Clip Studio Paint file: {0}")]
    NotClip(String),
    #[error("malformed .clip data: {0}")]
    Malformed(String),
}

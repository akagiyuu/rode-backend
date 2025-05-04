pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Aws(#[from] aws_sdk_s3::Error),

    #[error("{0}")]
    IO(#[from] std::io::Error),
}

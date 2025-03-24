use super::error::AppError;

pub type AppResult<T> = Result<T, AppError>;

// src/handlers/error.rs
use std::fmt;
use std::error::Error;
use warp::reject::Reject;

#[derive(Debug, Clone)]
pub enum ApiError {
    DatabaseError(String),
    ExternalServiceError(String),
    CacheError(String),
    ParseError(String),
}

// Implement the necessary traits
impl ApiError {
    pub fn database_error(msg: impl Into<String>) -> Self {
        ApiError::DatabaseError(msg.into())
    }

    pub fn external_error(msg: impl Into<String>) -> Self {
        ApiError::ExternalServiceError(msg.into())
    }

    pub fn cache_error(msg: impl Into<String>) -> Self {
        ApiError::CacheError(msg.into())
    }

    pub fn parse_error(msg: impl Into<String>) -> Self {
        ApiError::ParseError(msg.into())
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            ApiError::ExternalServiceError(msg) => write!(f, "External service error: {}", msg),
            ApiError::CacheError(msg) => write!(f, "Cache error: {}", msg),
            ApiError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

// This is required to make ApiError usable with Error trait objects
impl Error for ApiError {}

// This is required for warp's rejection handling
impl Reject for ApiError {}

// Explicitly implement Send and Sync
unsafe impl Send for ApiError {}
unsafe impl Sync for ApiError {}
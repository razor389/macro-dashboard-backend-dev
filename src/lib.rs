// src/lib.rs

// Re-export or define the top-level modules you need
pub mod services;
pub mod models;
pub mod handlers;
pub mod routes;

// Add this to src/lib.rs or a common module
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
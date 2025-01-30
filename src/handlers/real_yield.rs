// src/handlers/real_yield.rs
use warp::reply::Json;
use warp::Rejection;
use std::sync::Arc;
use crate::services::db::DbStore;
use super::error::ApiError;
use log::{info, error};

pub async fn get_real_yield(db: Arc<DbStore>) -> Result<Json, Rejection> {
    info!("Handling request to calculate real yield");

    let cache = db.get_market_cache().await.map_err(|e| {
        error!("Database error: {}", e);
        warp::reject::custom(ApiError::database_error(e.to_string()))
    })?;

    // Check if we have both required values
    if cache.tbill_yield == 0.0 || cache.inflation_rate == 0.0 {
        error!("Missing required data for real yield calculation");
        return Err(warp::reject::custom(ApiError::cache_error(
            "Missing required T-bill or inflation data".to_string()
        )));
    }

    let real_yield = cache.tbill_yield - cache.inflation_rate;
    info!("Calculated real yield: {}", real_yield);

    Ok(warp::reply::json(&real_yield))
}
// src/handlers/real_yield.rs
use warp::reply::with_status;
use warp::Rejection;
use std::sync::Arc;
use crate::services::db::DbStore;
use super::error::ApiError;
use log::{info, error, debug};
use serde_json::json;

pub async fn get_real_yield(db: Arc<DbStore>) -> Result<impl warp::Reply, Rejection> {
    info!("Handling request to calculate real yield");

    debug!("Attempting to get market cache");
    let cache = match db.get_market_cache().await {
        Ok(cache) => {
            debug!("Successfully retrieved market cache");
            cache
        },
        Err(e) => {
            error!("Failed to get market cache: {:?}", e);
            return Err(warp::reject::custom(ApiError::database_error(e.to_string())));
        }
    };

    // Check if we have both required values
    if cache.tbill_yield == 0.0 || cache.inflation_rate == 0.0 {
        error!("Missing required data for real yield calculation");
        return Err(warp::reject::custom(ApiError::cache_error(
            "Missing required T-bill or inflation data".to_string()
        )));
    }

    let real_yield = cache.tbill_yield - cache.inflation_rate;
    debug!("Calculated real yield: {}", real_yield);

    Ok(with_status(
        warp::reply::json(&json!({
            "real_yield": real_yield,
            "components": {
                "tbill_yield": cache.tbill_yield,
                "inflation_rate": cache.inflation_rate
            }
        })),
        warp::http::StatusCode::OK
    ))
}
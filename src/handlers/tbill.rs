// src/handlers/tbill.rs
use warp::reply::with_status;
use warp::Rejection;
use crate::services::treasury::fetch_tbill_data;
use log::{info, error, debug};
use std::sync::Arc;
use chrono::{Duration, Utc};
use crate::services::db::DbStore;
use super::error::ApiError;
use serde_json::json;

pub async fn get_tbill(db: Arc<DbStore>) -> Result<impl warp::Reply, Rejection> {
    info!("Handling request to get T-bill rate");

    debug!("Attempting to get market cache");
    let mut cache = match db.get_market_cache().await {
        Ok(cache) => {
            debug!("Successfully retrieved market cache");
            cache
        },
        Err(e) => {
            error!("Failed to get market cache: {:?}", e);
            return Err(warp::reject::custom(ApiError::database_error(e.to_string())));
        }
    };

    debug!("Current tbill cache timestamp: {:?}", cache.timestamps.treasury_data);
    if cache.timestamps.treasury_data < Utc::now() - Duration::hours(1) {
        info!("Cache expired, fetching new T-bill data");
        match fetch_tbill_data().await {
            Ok(rate) => {
                debug!("Successfully fetched new T-bill rate: {}", rate);
                cache.tbill_yield = rate;
                cache.timestamps.treasury_data = Utc::now();
                
                if let Err(e) = db.update_market_cache(&cache).await {
                    error!("Failed to update cache with new T-bill data: {}", e);
                    // Continue with old data if update fails
                }
            }
            Err(e) => {
                error!("Failed to fetch new T-bill data: {}", e);
                // Only reject if we have no cached data
                if cache.tbill_yield == 0.0 {
                    return Err(warp::reject::custom(ApiError::external_error(
                        format!("Failed to fetch T-bill data: {}", e)
                    )));
                }
            }
        }
    }

    debug!("Returning T-bill yield: {}", cache.tbill_yield);
    Ok(with_status(
        warp::reply::json(&json!({
            "rate": cache.tbill_yield
        })),
        warp::http::StatusCode::OK
    ))
}
// src/handlers/inflation.rs
use warp::reply::with_status;
use warp::Rejection;
use crate::services::bls::fetch_inflation_data;
use log::{info, error, debug};
use std::sync::Arc;
use chrono::{Duration, Utc};
use crate::services::db::DbStore;
use super::error::ApiError;
use serde_json::json;

pub async fn get_inflation(db: Arc<DbStore>) -> Result<impl warp::Reply, Rejection> {
    info!("Handling request to get inflation data");

    // Add debug logging for cache access
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

    debug!("Current inflation cache timestamp: {:?}", cache.timestamps.bls_data);
    if cache.timestamps.bls_data < Utc::now() - Duration::hours(1) {
        info!("Cache expired, fetching new inflation data");
        match fetch_inflation_data().await {
            Ok(rate) => {
                debug!("Successfully fetched new inflation rate: {}", rate);
                cache.inflation_rate = rate;
                cache.timestamps.bls_data = Utc::now();
                
                if let Err(e) = db.update_market_cache(&cache).await {
                    error!("Failed to update cache with new inflation data: {}", e);
                    // Continue with old data if update fails
                }
            }
            Err(e) => {
                error!("Failed to fetch new inflation data: {}", e);
                // Only reject if we have no cached data
                if cache.inflation_rate == 0.0 {
                    return Err(warp::reject::custom(ApiError::external_error(
                        format!("Failed to fetch inflation data: {}", e)
                    )));
                }
            }
        }
    }

    debug!("Returning inflation rate: {}", cache.inflation_rate);
    Ok(with_status(
        warp::reply::json(&json!({
            "rate": cache.inflation_rate
        })),
        warp::http::StatusCode::OK
    ))
}
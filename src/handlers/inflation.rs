// src/handlers/inflation.rs
use warp::reply::Json;
use warp::Rejection;
use crate::services::bls::fetch_inflation_data;
use log::{info, error};
use std::sync::Arc;
use chrono::{Duration, Utc};
use crate::services::db::DbStore;
use super::error::ApiError;

pub async fn get_inflation(db: Arc<DbStore>) -> Result<Json, Rejection> {
    info!("Handling request to get inflation data");

    let mut cache = db.get_market_cache().await.map_err(|e| {
        error!("Database error: {}", e);
        warp::reject::custom(ApiError::database_error(e.to_string()))
    })?;

    if cache.timestamps.bls_data < Utc::now() - Duration::hours(1) {
        info!("Cache expired, fetching new inflation data");
        match fetch_inflation_data().await {
            Ok(rate) => {
                cache.inflation_rate = rate;
                cache.timestamps.bls_data = Utc::now();
                if let Err(e) = db.update_market_cache(&cache).await {
                    error!("Failed to update cache: {}", e);
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

    Ok(warp::reply::json(&cache.inflation_rate))
}
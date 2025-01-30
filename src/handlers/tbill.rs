// src/handlers/tbill.rs
use warp::reply::Json;
use warp::Rejection;
use crate::services::treasury::fetch_tbill_data;
use log::{info, error};
use std::sync::Arc;
use chrono::{Duration, Utc};
use crate::services::db::DbStore;
use super::error::ApiError;

pub async fn get_tbill(db: Arc<DbStore>) -> Result<Json, Rejection> {
    info!("Handling request to get T-bill rate");

    let mut cache = db.get_market_cache().await.map_err(|e| {
        error!("Database error: {}", e);
        warp::reject::custom(ApiError::database_error(e.to_string()))
    })?;

    if cache.timestamps.treasury_data < Utc::now() - Duration::hours(1) {
        info!("Cache expired, fetching new T-bill data");
        match fetch_tbill_data().await {
            Ok(rate) => {
                cache.tbill_yield = rate;
                cache.timestamps.treasury_data = Utc::now();
                if let Err(e) = db.update_market_cache(&cache).await {
                    error!("Failed to update cache: {}", e);
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

    Ok(warp::reply::json(&cache.tbill_yield))
}
// src/handlers/tbill.rs
use warp::reply::Json;
use warp::Rejection;
use crate::services::treasury;
use log::{info, error};
use std::sync::Arc;
use chrono::{Duration, Utc};
use crate::services::db::DbStore;
use super::error::ApiError;

pub async fn get_tbill(db: Arc<DbStore>) -> Result<Json, Rejection> {
    info!("Handling request to get T-bill rate.");

    let mut cache = db.get_market_cache().await
        .map_err(|e| {
            error!("Failed to get market cache: {}", e);
            warp::reject::custom(ApiError::new(e.to_string()))
        })?;

    if cache.timestamps.treasury_data < Utc::now() - Duration::hours(1) {
        info!("Cache is old, fetching new T-bill data");
        match treasury::fetch_tbill_data().await {
            Ok(rate) => {
                cache.tbill_yield = rate;
                cache.timestamps.treasury_data = Utc::now();
                if let Err(e) = db.update_market_cache(&cache).await {
                    error!("Failed to update cache: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to fetch new T-bill data: {}", e);
                if cache.tbill_yield == 0.0 {
                    return Err(warp::reject::custom(ApiError::new(e.to_string())));
                }
            }
        }
    }

    Ok(warp::reply::json(&cache.tbill_yield))
}
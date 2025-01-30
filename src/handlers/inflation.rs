use warp::reply::Json;
use warp::Rejection;
use crate::services::bls::fetch_inflation_data;
use log::{info, error};
use std::sync::Arc;
use chrono::{Duration, Utc};
use crate::services::db::DbStore;
use super::error::ApiError;

// src/handlers/inflation.rs
pub async fn get_inflation(db: Arc<DbStore>) -> Result<Json, Rejection> {
    info!("Handling request to get inflation data.");

    let mut cache = db.get_market_cache().await.map_err(|e| {
        error!("Failed to get market cache: {}", e);
        warp::reject::custom(ApiError::new(e.to_string()))
    })?;

    if cache.timestamps.bls_data < Utc::now() - Duration::hours(1) {
        info!("Cache is old, fetching new inflation data");
        match fetch_inflation_data().await {
            Ok(rate) => {
                cache.inflation_rate = rate;
                cache.timestamps.bls_data = Utc::now();
                if let Err(e) = db.update_market_cache(&cache).await {
                    error!("Failed to update cache: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to fetch new inflation data: {}", e);
                if cache.inflation_rate == 0.0 {
                    return Err(warp::reject::custom(ApiError::new(e.to_string())));
                }
            }
        }
    }

    Ok(warp::reply::json(&cache.inflation_rate))
}

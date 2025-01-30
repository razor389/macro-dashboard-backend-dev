// src/handlers/long_term.rs
use warp::reply::Json;
use warp::Rejection;
use serde::Serialize;
use std::sync::Arc;
use crate::handlers::error::ApiError;
use crate::services::db::DbStore;
use crate::services::treasury_long::{fetch_20y_bond_yield, fetch_20y_tips_yield};
use log::{error, info};
use chrono::{Duration, Utc};

#[derive(Serialize)]
struct LongTermRatesResponse {
    bond_yield: f64,
    tips_yield: f64,
    real_tbill: f64,
}

pub async fn get_long_term_rates(db: Arc<DbStore>) -> Result<Json, Rejection> {
    info!("Handling request to get long-term rates");

    let mut cache = db.get_market_cache().await.map_err(|e| {
        error!("Database error: {}", e);
        warp::reject::custom(ApiError::database_error(e.to_string()))
    })?;

    if cache.timestamps.treasury_data < Utc::now() - Duration::hours(1) {
        info!("Cache expired, fetching new treasury data");
        
        let mut update_failed = false;
        
        match fetch_20y_bond_yield().await {
            Ok(rate) => {
                cache.bond_yield_20y = rate;
            }
            Err(e) => {
                error!("Failed to fetch 20y bond yield: {}", e);
                if cache.bond_yield_20y == 0.0 {
                    update_failed = true;
                }
            }
        }

        match fetch_20y_tips_yield().await {
            Ok(rate) => {
                cache.tips_yield_20y = rate;
            }
            Err(e) => {
                error!("Failed to fetch 20y TIPS yield: {}", e);
                if cache.tips_yield_20y == 0.0 {
                    update_failed = true;
                }
            }
        }

        if !update_failed {
            cache.timestamps.treasury_data = Utc::now();
            if let Err(e) = db.update_market_cache(&cache).await {
                error!("Failed to update cache: {}", e);
                // Continue with old data if update fails
            }
        } else {
            // Only reject if we have no data at all
            if cache.bond_yield_20y == 0.0 && cache.tips_yield_20y == 0.0 {
                return Err(warp::reject::custom(ApiError::external_error(
                    "Failed to fetch treasury yield data".to_string()
                )));
            }
        }
    }

    // Calculate real T-bill rate
    let real_tbill = if cache.tbill_yield != 0.0 && cache.inflation_rate != 0.0 {
        cache.tbill_yield - cache.inflation_rate
    } else {
        0.0 // Or another suitable default/fallback value
    };

    let response = LongTermRatesResponse {
        bond_yield: cache.bond_yield_20y,
        tips_yield: cache.tips_yield_20y,
        real_tbill,
    };

    Ok(warp::reply::json(&response))
}
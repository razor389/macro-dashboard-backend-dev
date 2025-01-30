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
struct LongTermRatesRaw {
    bond_yield: f64,
    tips_yield: f64,
    real_tbill: f64,
}

pub async fn get_long_term_rates(db: Arc<DbStore>) -> Result<Json, Rejection> {
    let mut cache = db.get_market_cache().await.map_err(|e| {
        error!("Failed to get market cache: {}", e);
        warp::reject::custom(ApiError::new(e.to_string()))
    })?;

    if cache.timestamps.treasury_data < Utc::now() - Duration::hours(1) {
        info!("Cache is old, fetching new treasury data");
        
        let mut update_failed = false;
        
        match fetch_20y_bond_yield().await {
            Ok(rate) => {
                cache.bond_yield_20y = rate;
            }
            Err(e) => {
                error!("Failed to fetch new 20y bond yield: {}", e);
                update_failed = true;
            }
        }

        match fetch_20y_tips_yield().await {
            Ok(rate) => {
                cache.tips_yield_20y = rate;
            }
            Err(e) => {
                error!("Failed to fetch new 20y TIPS yield: {}", e);
                update_failed = true;
            }
        }

        if !update_failed {
            cache.timestamps.treasury_data = Utc::now();
            if let Err(e) = db.update_market_cache(&cache).await {
                error!("Failed to update cache: {}", e);
            }
        }
    }

    let real_tbill = cache.tbill_yield - cache.inflation_rate;

    let response = LongTermRatesRaw {
        bond_yield: cache.bond_yield_20y,
        tips_yield: cache.tips_yield_20y,
        real_tbill,
    };

    Ok(warp::reply::json(&response))
}
// src/handlers/long_term.rs
use warp::reply::with_status;
use warp::Rejection;
use std::sync::Arc;
use crate::handlers::error::ApiError;
use crate::services::db::DbStore;
use crate::services::treasury_long::{fetch_20y_bond_yield, fetch_20y_tips_yield};
use log::{error, info, debug};
use chrono::{Duration, Utc};
use serde_json::json;

pub async fn get_long_term_rates(db: Arc<DbStore>) -> Result<impl warp::Reply, Rejection> {
    info!("Handling request to get long-term rates");

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

    debug!("Current treasury cache timestamp: {:?}", cache.timestamps.treasury_data);
    if cache.timestamps.treasury_data < Utc::now() - Duration::hours(1) {
        info!("Cache expired, fetching new treasury data");
        
        let mut update_failed = false;
        
        match fetch_20y_bond_yield().await {
            Ok(rate) => {
                debug!("Successfully fetched new 20y bond yield: {}", rate);
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
                debug!("Successfully fetched new 20y TIPS yield: {}", rate);
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

    debug!("Returning long-term rates: bond={}, tips={}, real_tbill={}", 
           cache.bond_yield_20y, cache.tips_yield_20y, real_tbill);
           
    Ok(with_status(
        warp::reply::json(&json!({
            "rates": {
                "bond_yield_20y": cache.bond_yield_20y,
                "tips_yield_20y": cache.tips_yield_20y,
                "real_tbill": real_tbill
            },
            "timestamps": {
                "treasury": cache.timestamps.treasury_data,
                "bls": cache.timestamps.bls_data
            }
        })),
        warp::http::StatusCode::OK
    ))
}
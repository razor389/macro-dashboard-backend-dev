// src/handlers/real_yield.rs
use std::sync::Arc;
use warp::reply::Json;
use warp::Rejection;
use crate::services::bls::fetch_inflation_data;
use crate::services::treasury::fetch_tbill_data;
use log::{info, error};
use crate::services::sheets::DbStore;

pub async fn get_real_yield(_db: Arc<DbStore>) -> Result<Json, Rejection> {
    info!("Handling request to calculate real yield.");

    let inflation = match fetch_inflation_data().await {
        Ok(inflation_rate) => {
            info!("Successfully fetched inflation rate: {}", inflation_rate);
            inflation_rate
        }
        Err(e) => {
            error!("Failed to fetch inflation rate: {}", e);
            return Err(warp::reject::not_found());
        }
    };

    let tbill_rate = match fetch_tbill_data().await {
        Ok(rate) => {
            info!("Successfully fetched T-bill rate: {}", rate);
            rate
        }
        Err(e) => {
            error!("Failed to fetch T-bill rate: {}", e);
            return Err(warp::reject::not_found());
        }
    };

    let real_yield = tbill_rate - inflation;
    info!("Calculated real yield: {}", real_yield);

    Ok(warp::reply::json(&real_yield))
}

// src/handlers/equity.rs
use warp::reply::Json;
use warp::Rejection;
use crate::services::equity::get_market_data;
use log::{error, info};

pub async fn get_equity_data() -> Result<Json, Rejection> {
    match get_market_data().await {
        Ok(data) => {
            info!("Successfully fetched market data: {:?}", data);
            Ok(warp::reply::json(&data))
        }
        Err(e) => {
            error!("Failed to fetch market data: {}", e);
            Err(warp::reject::not_found())
        }
    }
}
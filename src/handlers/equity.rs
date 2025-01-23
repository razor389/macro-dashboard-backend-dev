// src/handlers/equity.rs
use warp::reply::Json;
use warp::Rejection;
use crate::services::equity;
use log::{error, info};
use std::sync::Arc;
use crate::services::db::DbStore;

pub async fn get_equity_data(db: Arc<DbStore>) -> Result<Json, Rejection> {
    match equity::get_market_data(&db).await {
        Ok(data) => {
            info!("Successfully fetched market data");
            Ok(warp::reply::json(&data))
        }
        Err(e) => {
            error!("Failed to fetch market data: {}", e);
            Err(warp::reject::not_found())
        }
    }
}
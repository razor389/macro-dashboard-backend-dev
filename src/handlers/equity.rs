// src/handlers/equity.rs
use warp::reply::Json;
use warp::Rejection;
use crate::{handlers::error::ApiError, services::equity};
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

pub async fn get_equity_history(db: Arc<DbStore>) -> Result<Json, Rejection> {
    match equity::get_historical_data(&db).await {
        Ok(data) => {
            info!("Successfully fetched historical data");
            Ok(warp::reply::json(&data))
        }
        Err(e) => {
            error!("Failed to fetch historical data: {}", e);
            Err(warp::reject::not_found())
        }
    }
}

pub async fn get_equity_history_range(start_year: i32, end_year: i32, db: Arc<DbStore>) -> Result<Json, Rejection> {
    match equity::get_historical_data_range(&db, start_year, end_year).await {
        Ok(data) => {
            info!("Successfully fetched historical data range");
            Ok(warp::reply::json(&data))
        }
        Err(e) => {
            error!("Failed to fetch historical data range: {}", e);
            Err(warp::reject::not_found())
        }
    }
}

pub async fn get_market_metrics(db: Arc<DbStore>) -> Result<Json, Rejection> {
    match equity::get_market_metrics(&db).await {
        Ok(metrics) => {
            info!("Successfully calculated market metrics");
            Ok(warp::reply::json(&metrics))
        }
        Err(e) => {
            error!("Failed to calculate market metrics: {}", e);
            Err(warp::reject::custom(ApiError::database_error(e.to_string())))
        }
    }
}
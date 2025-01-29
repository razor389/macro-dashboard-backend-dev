// src/handlers/inflation.rs
use warp::reply::Json;
use warp::Rejection;
use crate::services::bls::fetch_inflation_data;
use log::{info, error};
use std::sync::Arc;
use crate::services::sheets::DbStore;

pub async fn get_inflation(_db: Arc<DbStore>) -> Result<Json, Rejection> {
    info!("Handling request to get inflation data.");

    match fetch_inflation_data().await {
        Ok(inflation) => {
            info!("Successfully fetched inflation data: {}", inflation);
            Ok(warp::reply::json(&inflation))
        }
        Err(e) => {
            error!("Failed to fetch inflation data: {}", e);
            Err(warp::reject::not_found())
        }
    }
}
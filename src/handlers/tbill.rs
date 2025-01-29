use warp::reply::Json;
use warp::Rejection;
use crate::services::treasury::fetch_tbill_data;
use log::{info, error};
use std::sync::Arc;
use crate::services::sheets::DbStore;

pub async fn get_tbill(_db: Arc<DbStore>) -> Result<Json, Rejection> {
    info!("Handling request to get T-bill rate.");

    match fetch_tbill_data().await {
        Ok(tbill_rate) => {
            info!("Successfully fetched T-bill rate: {}", tbill_rate);
            Ok(warp::reply::json(&tbill_rate))
        }
        Err(e) => {
            error!("Failed to fetch T-bill rate: {}", e);
            Err(warp::reject::not_found())
        }
    }
}

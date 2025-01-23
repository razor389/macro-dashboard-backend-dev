// src/handlers/long_term.rs
use warp::reply::Json;
use warp::Rejection;
use serde::Serialize;
use std::sync::Arc;
use crate::services::db::DbStore;
use crate::services::{
    bls::fetch_inflation_data,
    treasury::fetch_tbill_data,
    treasury_long::{fetch_20y_bond_yield, fetch_20y_tips_yield},
};

#[derive(Serialize)]
struct LongTermRatesRaw {
    bond_yield: f64,
    tips_yield: f64,
    real_tbill: f64,
}

pub async fn get_long_term_rates(_db: Arc<DbStore>) -> Result<Json, Rejection> {
    let inflation = fetch_inflation_data().await.map_err(|_| warp::reject::not_found())?;
    let nominal_tbill = fetch_tbill_data().await.map_err(|_| warp::reject::not_found())?;
    let real_tbill = nominal_tbill - inflation;
    let bond_yield = fetch_20y_bond_yield().await.map_err(|_| warp::reject::not_found())?;
    let tips_yield = fetch_20y_tips_yield().await.map_err(|_| warp::reject::not_found())?;

    let data = LongTermRatesRaw {
        bond_yield,
        tips_yield,
        real_tbill,
    };
    Ok(warp::reply::json(&data))
}
use std::sync::Arc;
use warp::Filter;
use crate::handlers::{equity::get_equity_data, equity::get_equity_history, equity::get_equity_history_range, inflation::get_inflation, 
                     long_term::get_long_term_rates, real_yield::get_real_yield, 
                     tbill::get_tbill};
use crate::services::db::DbStore;
use log::info;

pub fn routes(db: Arc<DbStore>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    info!("Configuring routes...");

    let db_filter = warp::any().map(move || db.clone());

    let inflation_route = warp::path!("api" / "v1" / "inflation")
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(get_inflation)
        .map(|reply| {
            info!("Inflation route was hit.");
            reply
        });

    let tbill_route = warp::path!("api" / "v1" / "tbill")
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(get_tbill)
        .map(|reply| {
            info!("T-bill route was hit.");
            reply
        });

    let real_yield_route = warp::path!("api" / "v1" / "real_yield")
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(get_real_yield)
        .map(|reply| {
            info!("Real yield route was hit.");
            reply
        });

    let long_term_route = warp::path!("api" / "v1" / "long_term_rates")
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(get_long_term_rates)
        .map(|reply| {
            info!("Long-term rates route was hit.");
            reply
        });

    let equity_route = warp::path!("api" / "v1" / "equity")
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(get_equity_data)
        .map(|reply| {
            info!("Equity route was hit.");
            reply
        });
    
        let equity_history_route = warp::path!("api" / "v1" / "equity" / "history" / "all")
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(get_equity_history)
        .map(|reply| {
            info!("Equity history route was hit.");
            reply
        });

    let equity_history_range_route = warp::path!("api" / "v1" / "equity" / "history" / i32 / i32)
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(get_equity_history_range)
        .map(|reply| {
            info!("Equity history range route was hit.");
            reply
        });
    
    info!("All routes configured successfully.");
    inflation_route
        .or(tbill_route)
        .or(real_yield_route)
        .or(long_term_route)
        .or(equity_route)
        .or(equity_history_route)
        .or(equity_history_range_route)
}
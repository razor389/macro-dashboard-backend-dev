// src/routes.rs
use std::sync::Arc;
use warp::reject::Rejection;
use crate::handlers::{equity::get_equity_data, equity::get_equity_history, equity::get_equity_history_range, inflation::get_inflation, 
                     long_term::get_long_term_rates, real_yield::get_real_yield, 
                     tbill::get_tbill};
use crate::services::db::DbStore;
use log::info;

use std::convert::Infallible;
use warp::{Filter, Reply};
use crate::handlers::error::ApiError;

// Add recovery handling for our custom errors
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = warp::http::StatusCode::NOT_FOUND;
        message = "Not Found";
    } else if let Some(api_error) = err.find::<ApiError>() {
        code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
        message = &api_error.message;
    } else {
        code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal Server Error";
    }

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "error": message,
        })),
        code,
    ))
}

pub fn routes(db: Arc<DbStore>) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    info!("Configuring routes...");

    let db_filter = warp::any().map(move || db.clone());

    let inflation_route = warp::path!("api" / "v1" / "inflation")
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(get_inflation);

    let tbill_route = warp::path!("api" / "v1" / "tbill")
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(get_tbill);

    let real_yield_route = warp::path!("api" / "v1" / "real_yield")
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(get_real_yield);

    let long_term_route = warp::path!("api" / "v1" / "long_term_rates")
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(get_long_term_rates);

    let equity_route = warp::path!("api" / "v1" / "equity")
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(get_equity_data);
    
    let equity_history_route = warp::path!("api" / "v1" / "equity" / "history" / "all")
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(get_equity_history);

    let equity_history_range_route = warp::path!("api" / "v1" / "equity" / "history" / i32 / i32)
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(get_equity_history_range);
    
    info!("All routes configured successfully.");

    inflation_route
        .or(tbill_route)
        .or(real_yield_route)
        .or(long_term_route)
        .or(equity_route)
        .or(equity_history_route)
        .or(equity_history_range_route)
        .recover(handle_rejection)
}
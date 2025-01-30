// src/routes.rs
use std::sync::Arc;
use std::convert::Infallible;
use warp::{Filter, Reply, Rejection};
use serde_json::json;
use log::{info, error, debug};

use crate::handlers::{
    equity::{get_equity_data, get_equity_history, get_equity_history_range, get_market_metrics}, error::ApiError, inflation::get_inflation, long_term::get_long_term_rates, real_yield::get_real_yield, tbill::get_tbill
};
use crate::services::db::DbStore;

/// Helper function to clone the db reference for each route
fn with_db(
    db: Arc<DbStore>,
) -> impl Filter<Extract = (Arc<DbStore>,), Error = Infallible> + Clone {
    warp::any().map(move || db.clone())
}

/// Handle all types of rejections that our API might encounter
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let (code, message) = if err.is_not_found() {
        (warp::http::StatusCode::NOT_FOUND, "Not Found".to_string())
    } else if let Some(api_error) = err.find::<ApiError>() {
        let code = match api_error {
            ApiError::DatabaseError(_) => warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::ExternalServiceError(_) => warp::http::StatusCode::BAD_GATEWAY,
            ApiError::CacheError(_) => warp::http::StatusCode::SERVICE_UNAVAILABLE,
            ApiError::ParseError(_) => warp::http::StatusCode::BAD_REQUEST,
        };
        (code, api_error.to_string())
    } else {
        error!("Unhandled rejection: {:?}", err);
        (
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error".to_string(),
        )
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&json!({
            "error": message,
        })),
        code,
    ))
}

/// Set up inflation route
fn inflation_route(
    db: Arc<DbStore>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "v1" / "inflation")
        .and(warp::get())
        .and(with_db(db))
        .and_then(get_inflation)
}

/// Set up T-bill route
fn tbill_route(
    db: Arc<DbStore>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "v1" / "tbill")
        .and(warp::get())
        .and(with_db(db))
        .and_then(get_tbill)
}

/// Set up real yield route
fn real_yield_route(
    db: Arc<DbStore>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "v1" / "real_yield")
        .and(warp::get())
        .and(with_db(db))
        .and_then(get_real_yield)
}

/// Set up long-term rates route
fn long_term_route(
    db: Arc<DbStore>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "v1" / "long_term_rates")
        .and(warp::get())
        .and(with_db(db))
        .and_then(get_long_term_rates)
}

/// Set up equity route
fn equity_route(
    db: Arc<DbStore>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "v1" / "equity")
        .and(warp::get())
        .and(with_db(db))
        .and_then(get_equity_data)
}

/// Set up equity history route
fn equity_history_route(
    db: Arc<DbStore>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "v1" / "equity" / "history" / "all")
        .and(warp::get())
        .and(with_db(db))
        .and_then(get_equity_history)
}

/// Set up equity history range route
fn equity_history_range_route(
    db: Arc<DbStore>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "v1" / "equity" / "history" / i32 / i32)
        .and(warp::get())
        .and(with_db(db))
        .and_then(get_equity_history_range)
}

fn market_metrics_route(
    db: Arc<DbStore>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "v1" / "equity" / "metrics")
        .and(warp::get())
        .and(with_db(db))
        .and_then(get_market_metrics)
}

/// Combine all routes into a single API
pub fn routes(db: Arc<DbStore>) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    info!("Configuring routes...");

    // Set up CORS with more permissive settings
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["Content-Type", "Authorization", "Accept"])
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .max_age(3600);

    // Health check route
    let health_route = warp::path!("health")
        .and(warp::get())
        .map(|| {
            debug!("Health check requested");
            warp::reply::json(&json!({"status": "ok"}))
        });

    // Combine all routes
    let api = health_route
        .or(inflation_route(db.clone()))
        .or(tbill_route(db.clone()))
        .or(real_yield_route(db.clone()))
        .or(long_term_route(db.clone()))
        .or(equity_route(db.clone()))
        .or(equity_history_route(db.clone()))
        .or(equity_history_range_route(db.clone()))
        .or(market_metrics_route(db.clone())); 

    // Add logging, CORS and error handling
    let api = api
        .with(warp::log("macro_dashboard_acm::api"))
        .with(cors)
        .recover(handle_rejection);

    info!("All routes configured successfully.");
    api
}
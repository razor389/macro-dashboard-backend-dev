use dotenv::dotenv;
use env_logger;
use log::{info, warn};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use warp::Filter;

mod handlers;
mod services;
mod routes;

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();
    info!("Logger initialized. Starting the application...");

    // Initialize database connection
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = services::db::DbStore::new(&database_url)
        .await
        .expect("Failed to connect to database");
    let db = Arc::new(db);
    let db_clone = db.clone();

    // Start background cache update service
    tokio::spawn(async move {
        services::equity::update_market_data(&db_clone)
            .await
            .expect("Failed to start cache update service");
    });

    // Get port from Heroku environment
    let port_str = env::var("PORT").unwrap_or_else(|_| {
        warn!("$PORT not set, defaulting to 3030");
        "3030".to_string()
    });
    
    let port: u16 = port_str.parse().expect("PORT must be a number");
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    info!("Will bind to: {}", addr);

    // Set up CORS
    let cors = warp::cors()
        .allow_any_origin()
        .allow_header("content-type")
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE"]);

    // Set up routes with db connection
    let api = routes::routes(db).with(cors);
    info!("Routes configured successfully with CORS.");

    info!("Starting server on {}", addr);
    warp::serve(api).run(addr).await;
}
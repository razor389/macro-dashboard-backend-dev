use env_logger;
use log::{info, warn};
use warp::Filter;
use std::env;
use std::net::SocketAddr;

mod handlers;
mod services;
mod routes;

#[tokio::main]
async fn main() {
    // Initialize the logger
    env_logger::init();
    info!("Logger initialized. Starting the application...");

    // Get port from Heroku environment, default to 3030
    let port_str = env::var("PORT").unwrap_or_else(|_| {
        warn!("$PORT not set, defaulting to 3030");
        "3030".to_string()
    });
    
    let port: u16 = port_str.parse().expect("PORT must be a number");
    info!("Using PORT: {}", port);

    // Bind to 0.0.0.0 for Heroku
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    info!("Will bind to: {}", addr);

    // Set up CORS
    let cors = warp::cors()
        .allow_any_origin()
        .allow_header("content-type")
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE"]);

    // Set up routes
    let api = routes::routes().with(cors);
    info!("Routes configured successfully with CORS.");

    // Start the server
    info!("Starting server on {}", addr);
    warp::serve(api)
        .run(addr)
        .await;
}
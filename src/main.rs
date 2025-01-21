mod handlers;
mod services;
mod routes;
use env_logger;
use log::info;
use warp::Filter;

#[tokio::main]
async fn main() {
    // Initialize the logger
    env_logger::init(); 
    info!("Logger initialized. Starting the application...");

    // Set up CORS
    let cors = warp::cors()
        .allow_any_origin()
        .allow_header("content-type")
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE"]);

    // Set up the routes with CORS
    let api = routes::routes().with(cors);
    info!("Routes configured successfully with CORS.");

    // Start the Warp server
    info!("Starting the Warp server on http://127.0.0.1:3030");
    warp::serve(api)
        .run(([127, 0, 0, 1], 3030))
        .await;

    info!("Server shut down gracefully.");
}

use dotenv::dotenv;
use env_logger;
use log::{info, warn, error};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use warp::Filter;
use tokio_cron_scheduler::{JobScheduler, Job};
use chrono_tz::US::Central;
use chrono::{Utc, TimeZone, Datelike}; // Added Datelike trait

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
    let scheduler_db = db.clone();

    // Initialize the scheduler
    let scheduler = JobScheduler::new().await.expect("Failed to create scheduler");
    
    // Schedule market data updates for 3:30 PM Central every day
    let daily_job = Job::new_async("0 30 15 * * *", move |_, _| {
        let db = scheduler_db.clone();
        Box::pin(async move {
            info!("Running scheduled market data update at 3:30 PM Central");
            match services::equity::get_market_data(&db).await {
                Ok(_) => info!("Successfully completed scheduled market data update"),
                Err(e) => error!("Failed to update market data: {}", e),
            }
        })
    }).expect("Failed to create daily job");

    // Add job to scheduler
    scheduler.add(daily_job).await.expect("Failed to add job to scheduler");

    // Start the scheduler
    scheduler.start().await.expect("Failed to start scheduler");

    // Start background service for immediate updates if needed
    tokio::spawn(async move {
        // Check if we need to catch up on any missed updates
        let now = Utc::now();
        let central_now = now.with_timezone(&Central);
        let target = Central.ymd(central_now.year(), central_now.month(), central_now.day())
            .and_hms_opt(15, 30, 0)
            .expect("Invalid time");

        // If we're starting after 3:30 PM Central and haven't updated today
        if central_now.time() > target.time() {
            let cache = db_clone.get_market_cache().await
                .expect("Failed to get market cache");
            
            let last_update = cache.timestamps.yahoo_price.with_timezone(&Central);
            if last_update.date_naive() < central_now.date_naive() {
                info!("Catching up on missed market update");
                if let Err(e) = services::equity::get_market_data(&db_clone).await {
                    error!("Failed to catch up on market data: {}", e);
                }
            }
        }
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
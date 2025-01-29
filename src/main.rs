// src/main.rs

use chrono::offset::LocalResult;
use dotenv::dotenv;
use env_logger;
use log::{info, warn, error};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use warp::Filter;
use tokio_cron_scheduler::{JobScheduler, Job};
use chrono_tz::US::Central;
use chrono::{Utc, TimeZone, Datelike};

use macro_dashboard_acm::services;
use macro_dashboard_acm::routes;

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();
    info!("Logger initialized. Starting the application...");

    // Initialize Google Sheets connection
    let spreadsheet_id = env::var("GOOGLE_SHEETS_ID")
        .expect("GOOGLE_SHEETS_ID must be set");
    // Instead of an API key, we use the service account JSON path
    let service_account_json_path = env::var("SERVICE_ACCOUNT_JSON")
        .expect("SERVICE_ACCOUNT_JSON must be set");

    let db = services::db::DbStore::new(&spreadsheet_id, &service_account_json_path)
        .await
        .expect("Failed to initialize Google Sheets connection");
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
        let now = Utc::now();
        let central_now = now.with_timezone(&Central);
        let target = match Central.with_ymd_and_hms(
            central_now.year(),
            central_now.month(),
            central_now.day(),
            15,
            30,
            0,
        ) {
            LocalResult::None => {
                panic!("Invalid date/time");
            }
            LocalResult::Ambiguous(dt1, dt2) => {
                panic!("Ambiguous local time: {} or {}", dt1, dt2);
            }
            LocalResult::Single(dt) => dt,
        };


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

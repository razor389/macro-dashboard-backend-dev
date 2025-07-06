use macro_dashboard_acm::services::db::DbStore;
use log::{info, error};
use env_logger;
use std::env;
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::init();
    
    info!("Testing Google Sheets connectivity and data reading...");
    
    let spreadsheet_id = env::var("GOOGLE_SHEETS_ID")
        .expect("GOOGLE_SHEETS_ID must be set");
    let service_account_json = env::var("SERVICE_ACCOUNT_JSON")
        .expect("SERVICE_ACCOUNT_JSON must be set");
    
    let db = DbStore::new(&spreadsheet_id, &service_account_json).await
        .expect("Failed to initialize database connection");
    
    // Test reading the market cache
    match db.get_market_cache().await {
        Ok(cache) => {
            info!("✓ Successfully read market cache from Google Sheets");
            info!("  Current S&P 500 price: {}", cache.current_sp500_price);
            info!("  Daily close price: {}", cache.daily_close_sp500_price);
            info!("  CAPE ratio: {} ({})", cache.current_cape, cache.cape_period);
            info!("  Latest monthly return: {} ({})", cache.latest_monthly_return, cache.latest_month);
            info!("  Last Yahoo update: {}", cache.timestamps.yahoo_price);
            info!("  Last YCharts update: {}", cache.timestamps.ycharts_data);
            info!("  Treasury rates - 20Y: {}, TIPS: {}, T-Bill: {}", 
                  cache.bond_yield_20y, cache.tips_yield_20y, cache.tbill_yield);
        }
        Err(e) => {
            error!("✗ Failed to read market cache: {}", e);
        }
    }
    
    Ok(())
}
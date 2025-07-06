use macro_dashboard_acm::services::{db::DbStore, equity::get_market_data};
use log::{info, error};
use env_logger;
use std::{env, sync::Arc};
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::init();
    
    info!("Testing market cache update in Google Sheets...");
    
    let spreadsheet_id = env::var("GOOGLE_SHEETS_ID")
        .expect("GOOGLE_SHEETS_ID must be set");
    let service_account_json = env::var("SERVICE_ACCOUNT_JSON")
        .expect("SERVICE_ACCOUNT_JSON must be set");
    
    let db = Arc::new(DbStore::new(&spreadsheet_id, &service_account_json).await
        .expect("Failed to initialize database connection"));
    
    // Read initial cache
    let initial_cache = db.get_market_cache().await
        .expect("Failed to read initial market cache");
    
    info!("Initial state:");
    info!("  Current S&P 500 price: {}", initial_cache.current_sp500_price);
    info!("  Last Yahoo update: {}", initial_cache.timestamps.yahoo_price);
    info!("  Last YCharts update: {}", initial_cache.timestamps.ycharts_data);
    
    // Run market data update
    info!("Running market data update...");
    match get_market_data(&db).await {
        Ok(_) => {
            info!("✓ Market data update completed successfully");
            
            // Read updated cache
            let updated_cache = db.get_market_cache().await
                .expect("Failed to read updated market cache");
                
            info!("Updated state:");
            info!("  Current S&P 500 price: {}", updated_cache.current_sp500_price);
            info!("  Last Yahoo update: {}", updated_cache.timestamps.yahoo_price);
            info!("  Last YCharts update: {}", updated_cache.timestamps.ycharts_data);
            
            // Check if data was actually updated
            if updated_cache.current_sp500_price != initial_cache.current_sp500_price {
                info!("✓ S&P 500 price was updated from {} to {}", 
                      initial_cache.current_sp500_price, updated_cache.current_sp500_price);
            } else {
                info!("⚠ S&P 500 price unchanged (may be same as current price)");
            }
            
            if updated_cache.timestamps.yahoo_price != initial_cache.timestamps.yahoo_price {
                info!("✓ Yahoo timestamp was updated");
            } else {
                info!("⚠ Yahoo timestamp unchanged");
            }
            
            if updated_cache.timestamps.ycharts_data != initial_cache.timestamps.ycharts_data {
                info!("✓ YCharts timestamp was updated");
            } else {
                info!("⚠ YCharts timestamp unchanged");
            }
        }
        Err(e) => {
            error!("✗ Market data update failed: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}
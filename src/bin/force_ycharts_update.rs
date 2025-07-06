use macro_dashboard_acm::services::{db::DbStore, equity::get_market_data};
use log::{info, error};
use env_logger;
use std::{env, sync::Arc};
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::init();
    
    info!("Forcing YCharts update by simulating daily update...");
    
    let spreadsheet_id = env::var("GOOGLE_SHEETS_ID")
        .expect("GOOGLE_SHEETS_ID must be set");
    let service_account_json = env::var("SERVICE_ACCOUNT_JSON")
        .expect("SERVICE_ACCOUNT_JSON must be set");
    
    let db = Arc::new(DbStore::new(&spreadsheet_id, &service_account_json).await
        .expect("Failed to initialize database connection"));
    
    // Read initial cache
    let initial_cache = db.get_market_cache().await
        .expect("Failed to read initial market cache");
    
    info!("Before update:");
    info!("  Latest monthly return: {} ({})", initial_cache.latest_monthly_return, initial_cache.latest_month);
    info!("  CAPE: {} ({})", initial_cache.current_cape, initial_cache.cape_period);
    info!("  Last YCharts update: {}", initial_cache.timestamps.ycharts_data);
    
    // Force a complete market data update (this should include YCharts if it's the daily update time)
    info!("Running complete market data update...");
    match get_market_data(&db).await {
        Ok(_) => {
            info!("✓ Market data update completed");
            
            // Read updated cache
            let updated_cache = db.get_market_cache().await
                .expect("Failed to read updated market cache");
                
            info!("After update:");
            info!("  Latest monthly return: {} ({})", updated_cache.latest_monthly_return, updated_cache.latest_month);
            info!("  CAPE: {} ({})", updated_cache.current_cape, updated_cache.cape_period);
            info!("  Last YCharts update: {}", updated_cache.timestamps.ycharts_data);
            
            // Check if YCharts data was updated
            if updated_cache.timestamps.ycharts_data != initial_cache.timestamps.ycharts_data {
                info!("✓ YCharts timestamp was updated!");
            } else {
                info!("⚠ YCharts timestamp unchanged - may not be daily update time (3:30 PM Central)");
            }
            
            if updated_cache.latest_month != initial_cache.latest_month || 
               updated_cache.latest_monthly_return != initial_cache.latest_monthly_return {
                info!("✓ Monthly return data was updated!");
            } else {
                info!("⚠ Monthly return data unchanged");
            }
        }
        Err(e) => {
            error!("✗ Market data update failed: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}
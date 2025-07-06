use macro_dashboard_acm::services::{db::DbStore, equity::{fetch_ycharts_data, update_monthly_data}};
use log::{info, error};
use env_logger;
use std::{env, sync::Arc};
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::init();
    
    info!("Testing YCharts monthly return update functionality...");
    
    let spreadsheet_id = env::var("GOOGLE_SHEETS_ID")
        .expect("GOOGLE_SHEETS_ID must be set");
    let service_account_json = env::var("SERVICE_ACCOUNT_JSON")
        .expect("SERVICE_ACCOUNT_JSON must be set");
    
    let db = Arc::new(DbStore::new(&spreadsheet_id, &service_account_json).await
        .expect("Failed to initialize database connection"));
    
    // Test YCharts data fetching
    info!("Fetching YCharts data...");
    match fetch_ycharts_data().await {
        Ok(ycharts_data) => {
            info!("✓ Successfully fetched YCharts data");
            
            // Check monthly return data
            if let Some((month, return_value)) = &ycharts_data.monthly_return {
                info!("Found monthly return: {} = {}", month, return_value);
                
                // Test updating monthly data
                info!("Testing monthly data update...");
                match update_monthly_data(&db, month, *return_value).await {
                    Ok(_) => {
                        info!("✓ Successfully updated monthly data for {}", month);
                    }
                    Err(e) => {
                        error!("✗ Failed to update monthly data: {}", e);
                    }
                }
            } else {
                error!("✗ No monthly return data found in YCharts response");
            }
            
            // Check other data
            info!("Other YCharts data:");
            info!("  CAPE: {} ({})", ycharts_data.cape.0, ycharts_data.cape.1);
            info!("  Quarterly dividends: {} entries", ycharts_data.quarterly_dividends.len());
            info!("  EPS actual: {} entries", ycharts_data.eps_actual.len());
            info!("  EPS estimated: {} entries", ycharts_data.eps_estimated.len());
        }
        Err(e) => {
            error!("✗ Failed to fetch YCharts data: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}
use macro_dashboard_acm::services::equity::fetch_sp500_price;
use log::{info, error};
use env_logger;
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::init();
    
    info!("Testing Yahoo Finance S&P 500 price fetching...");
    
    match fetch_sp500_price().await {
        Ok(price) => {
            info!("SUCCESS: Yahoo Finance S&P 500 price: {}", price);
        }
        Err(e) => {
            error!("ERROR: Failed to fetch Yahoo Finance price: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}
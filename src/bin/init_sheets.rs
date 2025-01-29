// src/bin/init_sheets.rs
use dotenv::dotenv;
use std::{error::Error, fs};
use serde_json::Value;
use chrono::Utc;
use macro_dashboard_acm::services::sheets::{SheetsStore, SheetsConfig, MarketCache, QuarterlyData};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    // Get configuration from environment
    let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")?;
    let api_key = std::env::var("GOOGLE_API_KEY")?;

    let config = SheetsConfig {
        spreadsheet_id,
        api_key,
    };

    // Initialize sheets store
    let store = SheetsStore::new(config);

    // Read initialization data
    let init_data: Value = serde_json::from_str(
        &fs::read_to_string("config/market_init.json")?
    )?;

    // Initialize market cache
    let market_cache = MarketCache {
        timestamp_yahoo: Utc::now().to_rfc3339(),
        timestamp_ycharts: Utc::now().to_rfc3339(),
        daily_close_sp500_price: 0.0,
        current_sp500_price: 0.0,
        current_cape: init_data["cape"]["value"].as_f64().unwrap(),
        cape_period: init_data["cape"]["period"].as_str().unwrap().to_string(),
    };

    store.update_market_cache(&market_cache).await?;

    // Initialize quarterly data
    let mut quarterly_data = Vec::new();

    // Process earnings data
    for (quarter, value) in init_data["quarterly_earnings"].as_object().unwrap() {
        if let Some(num) = value.as_f64() {
            quarterly_data.push(QuarterlyData {
                quarter: quarter.clone(),
                dividend: None,
                eps_actual: Some(num),
                eps_estimated: None,
            });
        }
    }

    // Process dividend data
    for (quarter, value) in init_data["quarterly_dividends"].as_object().unwrap() {
        if let Some(num) = value.as_f64() {
            if let Some(existing) = quarterly_data.iter_mut().find(|q| q.quarter == *quarter) {
                existing.dividend = Some(num);
            } else {
                quarterly_data.push(QuarterlyData {
                    quarter: quarter.clone(),
                    dividend: Some(num),
                    eps_actual: None,
                    eps_estimated: None,
                });
            }
        }
    }

    // Process earnings estimates
    for (quarter, value) in init_data["earnings_estimates"].as_object().unwrap() {
        if let Some(num) = value.as_f64() {
            if let Some(existing) = quarterly_data.iter_mut().find(|q| q.quarter == *quarter) {
                existing.eps_estimated = Some(num);
            } else {
                quarterly_data.push(QuarterlyData {
                    quarter: quarter.clone(),
                    dividend: None,
                    eps_actual: None,
                    eps_estimated: Some(num),
                });
            }
        }
    }

    store.update_quarterly_data(&quarterly_data).await?;

    println!("Sheet initialization complete!");
    Ok(())
}
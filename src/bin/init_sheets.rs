// src/bin/init_sheets.rs

use dotenv::dotenv;
use std::{error::Error, fs};
use serde_json::Value;
use chrono::Utc;
use std::env;

use macro_dashboard_acm::services::sheets::{SheetsStore, SheetsConfig, RawMarketCache};
use macro_dashboard_acm::models::QuarterlyData;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    // e.g. .env has GOOGLE_SHEETS_ID=...  SERVICE_ACCOUNT_JSON=service_account.json
    let spreadsheet_id = env::var("GOOGLE_SHEETS_ID")?;
    let sa_json = env::var("SERVICE_ACCOUNT_JSON")?;

    let config = SheetsConfig {
        spreadsheet_id,
        service_account_json_path: sa_json,
    };

    let store = SheetsStore::new(config);

    // Read initialization data
    let init_data: Value = serde_json::from_str(
        &fs::read_to_string("config/market_init.json")?
    )?;

    // Fill a RawMarketCache with your data
    let market_cache = RawMarketCache {
        timestamp_yahoo: Utc::now().to_rfc3339(),
        timestamp_ycharts: Utc::now().to_rfc3339(),
        timestamp_treasury: Utc::now().to_rfc3339(),
        timestamp_bls: Utc::now().to_rfc3339(),
        daily_close_sp500_price: 0.0,
        current_sp500_price: 0.0,
        current_cape: init_data["cape"]["value"].as_f64().unwrap(),
        cape_period: init_data["cape"]["period"].as_str().unwrap().to_string(),
        tips_yield_20y: 0.0,
        bond_yield_20y: 0.0,
        tbill_yield: 0.0,
        inflation_rate: 0.0,
    };

    store.update_market_cache(&market_cache).await?;

    // Build QuarterlyData rows
    let mut quarterly_data = Vec::new();

    // Process earnings data
    if let Some(q_earnings) = init_data["quarterly_earnings"].as_object() {
        for (quarter, value) in q_earnings {
            if let Some(num) = value.as_f64() {
                quarterly_data.push(QuarterlyData {
                    quarter: quarter.clone(),
                    dividend: None,
                    eps_actual: Some(num),
                    eps_estimated: None,
                });
            }
        }
    }

    // Process dividend data
    if let Some(q_divs) = init_data["quarterly_dividends"].as_object() {
        for (quarter, value) in q_divs {
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
    }

    // Process earnings estimates
    if let Some(q_est) = init_data["earnings_estimates"].as_object() {
        for (quarter, value) in q_est {
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
    }

    // Now upload them to the sheet
    store.update_quarterly_data(&quarterly_data).await?;

    println!("Sheet initialization complete!");
    Ok(())
}

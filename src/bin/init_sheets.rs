// src/bin/init_sheets.rs

use dotenv::dotenv;
use std::{error::Error, fs};
use serde_json::Value;
use chrono::Utc;
use std::env;
use log::{info, error};
use macro_dashboard_acm::models::MonthlyData;

use macro_dashboard_acm::services::{
    sheets::{SheetsStore, SheetsConfig, RawMarketCache},
    bls::fetch_inflation_data,
    treasury::fetch_tbill_data,
    treasury_long::{fetch_20y_bond_yield, fetch_20y_tips_yield}
};
use macro_dashboard_acm::models::QuarterlyData;

async fn initialize_monthly_data(store: &SheetsStore) -> Result<(), Box<dyn Error>> {
    info!("Initializing monthly return data...");
    
    let init_data: Value = serde_json::from_str(
        &fs::read_to_string("config/market_init.json")?
    )?;

    let mut monthly_data: Vec<MonthlyData> = Vec::new();  // Explicitly type the vector

    if let Some(returns) = init_data["monthly_returns"].as_object() {
        for (month, value) in returns {
            if let Some(return_value) = value.as_f64() {
                monthly_data.push(MonthlyData {
                    month: month.clone(),
                    total_return: return_value,
                });
            }
        }
    }

    // Sort monthly data by date
    monthly_data.sort_by(|a, b| a.month.cmp(&b.month));

    info!("Uploading {} monthly records...", monthly_data.len());
    store.update_monthly_data(&monthly_data[..]).await?;
    info!("Monthly data initialized successfully");

    Ok(())
}

async fn initialize_market_data() -> Result<RawMarketCache, Box<dyn Error>> {
    info!("Fetching initial market data...");
    
    // Read initialization data for static values
    let init_data: Value = serde_json::from_str(
        &fs::read_to_string("config/market_init.json")?
    )?;

    // Fetch real-time data
    let inflation_rate = match fetch_inflation_data().await {
        Ok(rate) => {
            info!("Successfully fetched inflation rate: {}", rate);
            rate
        },
        Err(e) => {
            error!("Failed to fetch inflation rate: {}", e);
            0.0
        }
    };

    let tbill_yield = match fetch_tbill_data().await {
        Ok(rate) => {
            info!("Successfully fetched T-bill yield: {}", rate);
            rate
        },
        Err(e) => {
            error!("Failed to fetch T-bill yield: {}", e);
            0.0
        }
    };

    let bond_yield_20y = match fetch_20y_bond_yield().await {
        Ok(rate) => {
            info!("Successfully fetched 20y bond yield: {}", rate);
            rate
        },
        Err(e) => {
            error!("Failed to fetch 20y bond yield: {}", e);
            0.0
        }
    };

    let tips_yield_20y = match fetch_20y_tips_yield().await {
        Ok(rate) => {
            info!("Successfully fetched 20y TIPS yield: {}", rate);
            rate
        },
        Err(e) => {
            error!("Failed to fetch 20y TIPS yield: {}", e);
            0.0
        }
    };

    // -- Find the latest monthly return from config/market_init.json --
    let (latest_month, latest_monthly_return) = if let Some(monthly_returns) = init_data["monthly_returns"].as_object() {
        // Convert to a vec of (String, f64) so we can sort
        let mut pairs: Vec<(String, f64)> = monthly_returns.iter()
            .filter_map(|(m, val)| val.as_f64().map(|r| (m.clone(), r)))
            .collect();

        // Sort by month key "YYYY-MM"
        pairs.sort_by(|a, b| a.0.cmp(&b.0));

        // Last element is the "latest"
        if let Some((m, r)) = pairs.last() {
            (m.clone(), *r)
        } else {
            // Fallback default if no data
            ("".to_string(), 0.0)
        }
    } else {
        // Fallback default if JSON is missing monthly_returns
        ("".to_string(), 0.0)
    };

    let now = Utc::now().to_rfc3339();

    Ok(RawMarketCache {
        timestamp_yahoo: now.clone(),
        timestamp_ycharts: now.clone(),
        timestamp_treasury: now.clone(),
        timestamp_bls: now.clone(),
        daily_close_sp500_price: 0.0,
        current_sp500_price: 0.0,
        current_cape: init_data["cape"]["value"].as_f64().unwrap_or(0.0),
        cape_period: init_data["cape"]["period"].as_str().unwrap_or("").to_string(),
        tips_yield_20y,
        bond_yield_20y,
        tbill_yield,
        inflation_rate,
        latest_monthly_return,
        latest_month,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    env_logger::init();

    info!("Starting sheet initialization process...");

    let spreadsheet_id = env::var("GOOGLE_SHEETS_ID")?;
    let sa_json = env::var("SERVICE_ACCOUNT_JSON")?;

    let config = SheetsConfig {
        spreadsheet_id,
        service_account_json_path: sa_json,
    };

    let store = SheetsStore::new(config);

    // Initialize market cache with real data
    info!("Initializing market cache with real-time data...");
    let market_cache = initialize_market_data().await?;
    store.update_market_cache(&market_cache).await?;
    info!("Market cache initialized successfully");

    // Build QuarterlyData rows
    info!("Processing quarterly data...");
    let init_data: Value = serde_json::from_str(
        &fs::read_to_string("config/market_init.json")?
    )?;

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

    // Update quarterly data
    info!("Updating quarterly data...");
    store.update_quarterly_data(&quarterly_data).await?;

    initialize_monthly_data(&store).await?;
    
    info!("Sheet initialization complete!");
    Ok(())
}
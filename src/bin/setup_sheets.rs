// src/bin/setup_sheets.rs

use csv::Reader;
use dotenv::dotenv;
use log::{info, error};
use macro_dashboard_acm::models::HistoricalRecord;
use serde_json::json;
use std::{error::Error, fs::File};
use std::env;
use macro_dashboard_acm::services::sheets::{SheetsStore, SheetsConfig};

async fn create_sheet_if_not_exists(store: &SheetsStore, sheet_name: &str, headers: Vec<&str>) -> Result<(), Box<dyn Error>> {
    // First try to read the sheet - if it fails, we'll create it
    let token = store.get_auth_token().await?;
    let spreadsheet_id = &store.config.spreadsheet_id;
    
    // Create a request to add a sheet
    let add_sheet_request = json!({
        "requests": [{
            "addSheet": {
                "properties": {
                    "title": sheet_name,
                    "gridProperties": {
                        "rowCount": 1000,
                        "columnCount": headers.len(),
                        "frozenRowCount": 1
                    }
                }
            }
        }]
    });

    // Try to add the sheet
    let client = reqwest::Client::new();
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}/batchUpdate",
        spreadsheet_id
    );

    let response = client
        .post(&url)
        .bearer_auth(&token)
        .json(&add_sheet_request)
        .send()
        .await;

    // If sheet already exists, that's fine
    if let Err(e) = response {
        if !e.to_string().contains("already exists") {
            error!("Error creating sheet: {}", e);
            return Err(Box::new(e));
        }
    }

    // Now add headers
    let range = format!("{}!A1:{}{}", sheet_name, (b'A' + (headers.len() - 1) as u8) as char, 1);
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?valueInputOption=RAW",
        spreadsheet_id,
        range
    );

    let values = vec![headers.iter().map(|&s| s.to_string()).collect::<Vec<_>>()];
    let body = json!({
        "values": values,
    });

    client
        .put(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;

    info!("Sheet '{}' setup complete", sheet_name);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    env_logger::init();

    let spreadsheet_id = env::var("GOOGLE_SHEETS_ID")?;
    let sa_json = env::var("SERVICE_ACCOUNT_JSON")?;

    let config = SheetsConfig {
        spreadsheet_id,
        service_account_json_path: sa_json,
    };

    let store = SheetsStore::new(config);

    // Create MarketCache sheet
    create_sheet_if_not_exists(
        &store,
        "MarketCache",
        vec![
            "timestamp_yahoo",
            "timestamp_ycharts",
            "daily_close_sp500_price",
            "current_sp500_price",
            "current_cape",
            "cape_period"
        ],
    ).await?;

    // Create QuarterlyData sheet
    create_sheet_if_not_exists(
        &store,
        "QuarterlyData",
        vec![
            "quarter",
            "dividend",
            "eps_actual",
            "eps_estimated"
        ],
    ).await?;

    // Create HistoricalData sheet
    create_sheet_if_not_exists(
        &store,
        "HistoricalData",
        vec![
            "year",
            "sp500_price",
            "dividend",
            "eps",
            "cape"
        ],
    ).await?;

    // Now load historical data from CSV
    info!("Loading historical data from CSV...");
    let file = File::open("data/stk_mkt.csv")?;
    let mut rdr = Reader::from_reader(file);

    let mut historical_records = Vec::new();

    for result in rdr.records() {
        let record = result?;
        if &record[0] == "Year" {
            continue;
        }

        historical_records.push(HistoricalRecord {
            year: record[0].trim().parse()?,
            sp500_price: record[1].trim().parse()?,
            dividend: record[2].trim().parse()?,
            eps: record[4].trim().parse().unwrap_or(0.0),
            cape: record[5].trim().parse()?,
        });
    }

    info!("Uploading {} historical records...", historical_records.len());
    for hr in historical_records {
        store.update_historical_record(&hr).await?;
    }

    info!("Sheet setup and data loading complete!");
    Ok(())
}
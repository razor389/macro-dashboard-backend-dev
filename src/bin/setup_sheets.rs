//src/bin/setup_sheets.rs
use dotenv::dotenv;
use log::{info, error};
use macro_dashboard_acm::models::HistoricalRecord;
use serde_json::{Value, json};
use std::{error::Error, fs::File};
use std::env;
use macro_dashboard_acm::services::sheets::{SheetsStore, SheetsConfig};


async fn verify_spreadsheet_access(store: &SheetsStore) -> Result<(), Box<dyn Error>> {
    let token = store.get_auth_token().await?;
    let client = reqwest::Client::new();
    
    // Note: URL format is specifically for Google Sheets API v4
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}?includeGridData=false",
        store.config.spreadsheet_id
    );

    info!("Verifying spreadsheet access with token: {}...", &token[..10]);
    let response = client
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        error!("Failed to access spreadsheet: {} - {}", status, error_text);
        return Err(format!("Failed to access spreadsheet: {} - {}", status, error_text).into());
    }

    info!("Successfully verified spreadsheet access");
    Ok(())
}

async fn create_sheet_if_not_exists(store: &SheetsStore, sheet_name: &str, headers: Vec<&str>) -> Result<(), Box<dyn Error>> {
    let token = store.get_auth_token().await?;
    let client = reqwest::Client::new();
    
    // First check if sheet exists
    let metadata_url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}?includeGridData=false",
        store.config.spreadsheet_id
    );

    info!("Checking if sheet '{}' exists...", sheet_name);
    let response = client
        .get(&metadata_url)
        .bearer_auth(&token)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        return Err(format!("Failed to get spreadsheet info: {} - {}", status, error_text).into());
    }

    let spreadsheet_info: Value = response.json().await?;
    let sheet_exists = spreadsheet_info["sheets"]
        .as_array()
        .and_then(|sheets| {
            sheets.iter().find(|sheet| {
                sheet["properties"]["title"].as_str() == Some(sheet_name)
            })
        })
        .is_some();

    if !sheet_exists {
        info!("Creating new sheet '{}'...", sheet_name);
        let batch_update_url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}:batchUpdate",
            store.config.spreadsheet_id
        );

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

        info!("Sending request to create sheet: {}", batch_update_url);
        let response = client
            .post(&batch_update_url)
            .header("Content-Type", "application/json")
            .bearer_auth(&token)
            .json(&add_sheet_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(format!("Failed to create sheet: {} - {}", status, error_text).into());
        }
        
        info!("Sheet created successfully");
    } else {
        info!("Sheet '{}' already exists", sheet_name);
    }

    // Now set the headers directly without clearing first
    info!("Setting headers for '{}'...", sheet_name);
    let update_url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}!A1:{}1",
        store.config.spreadsheet_id,
        sheet_name,
        (b'A' + (headers.len() - 1) as u8) as char
    );

    let body = json!({
        "values": [headers],
        "majorDimension": "ROWS"
    });

    let response = client
        .put(&update_url)
        .header("Content-Type", "application/json")
        .query(&[("valueInputOption", "RAW")])
        .bearer_auth(token)
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        return Err(format!("Failed to update headers: {} - {}", status, error_text).into());
    }

    info!("Successfully set up sheet '{}'", sheet_name);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    env_logger::init();

    info!("Starting sheet setup process...");

    let spreadsheet_id = env::var("GOOGLE_SHEETS_ID")?;
    let sa_json = env::var("SERVICE_ACCOUNT_JSON")?;

    info!("Using spreadsheet ID: {}", spreadsheet_id);
    info!("Service account JSON path: {}", sa_json);

    let config = SheetsConfig {
        spreadsheet_id,
        service_account_json_path: sa_json,
    };

    let store = SheetsStore::new(config);

    // First verify we can access the spreadsheet
    verify_spreadsheet_access(&store).await?;

    // Setup sheets with headers
    let sheets_to_create = [
        ("MarketCache", vec![
            "timestamp_yahoo",
            "timestamp_ycharts",
            "daily_close_sp500_price",
            "current_sp500_price",
            "current_cape",
            "cape_period"
        ]),
        ("QuarterlyData", vec![
            "quarter",
            "dividend",
            "eps_actual",
            "eps_estimated"
        ]),
        ("HistoricalData", vec![
            "year",
            "sp500_price",
            "dividend",
            "dividend_yield",
            "eps",
            "cape",
            "inflation",
            "total_return",
            "cumulative_return"
        ])
    ];

    for (sheet_name, headers) in sheets_to_create.iter() {
        create_sheet_if_not_exists(&store, sheet_name, headers.clone()).await?;
    }

    // Load and upload historical data
    info!("Loading historical data from CSV...");
    // Helper function to safely parse float values - fixed signature to use &str
    fn parse_float(s: &str, field: &str) -> Result<f64, Box<dyn std::error::Error>> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Ok(0.0); // or whatever default value makes sense
        }
        trimmed.parse::<f64>().map_err(|e| {
            format!("Error parsing {} value '{}': {}", field, trimmed, e).into()
        })
    }

    let file = File::open("data/stk_mkt.csv")?;
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(file);

    let mut row_number = 0;
    let mut historical_records = Vec::new();

    for result in rdr.records() {
        row_number += 1;
        let record = result?;
        
        // Skip header row
        if &record[0] == "Year" {
            continue;
        }

        // Log the current row being processed
        info!("Processing row {}: {:?}", row_number, record);

        let record_attempt = HistoricalRecord {
            year: record[0].trim().parse().map_err(|e| {
                format!("Error parsing year '{}': {}", &record[0], e)
            })?,
            sp500_price: parse_float(&record[1], "SP500 price")?,
            dividend: parse_float(&record[2], "dividend")?,
            dividend_yield: parse_float(&record[3], "dividend yield")?,
            eps: parse_float(&record[4], "EPS")?,
            cape: parse_float(&record[5], "CAPE")?,
            inflation: parse_float(&record[6], "inflation")?,
            total_return: parse_float(&record[7], "total return")?,
            cumulative_return: parse_float(&record[8], "cumulative return")?,
        };

        historical_records.push(record_attempt);
    }

    info!("Successfully parsed {} records", historical_records.len());

    info!("Uploading {} historical records in bulk...", historical_records.len());
    store.bulk_upload_historical_records(&historical_records).await?;
    info!("Historical data upload complete!");
    info!("Sheet setup and data loading complete!");
    Ok(())
}
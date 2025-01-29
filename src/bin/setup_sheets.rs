// src/bin/setup_sheets.rs

use csv::Reader;
use dotenv::dotenv;
use macro_dashboard_acm::models::HistoricalRecord;
use std::{error::Error, fs::File};
use std::env;
use macro_dashboard_acm::services::sheets::{SheetsStore, SheetsConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let spreadsheet_id = env::var("GOOGLE_SHEETS_ID")?;
    let sa_json = env::var("SERVICE_ACCOUNT_JSON")?;

    let config = SheetsConfig {
        spreadsheet_id,
        service_account_json_path: sa_json,
    };

    let store = SheetsStore::new(config);

    // Read CSV file
    let file = File::open("data/stk_mkt.csv")?;
    let mut rdr = Reader::from_reader(file);

    let mut historical_records = Vec::new();

    for result in rdr.records() {
        let record = result?;
        // If the CSV has a header named "Year", skip it
        if &record[0] == "Year" {
            continue;
        }

        // Parse each row into a HistoricalRecord
        historical_records.push(HistoricalRecord {
            year: record[0].trim().parse()?,
            sp500_price: record[1].trim().parse()?,
            dividend: record[2].trim().parse()?,
            eps: record[3].trim().parse().unwrap_or(0.0),
            cape: record[4].trim().parse()?,
        });
    }

    // Update each row in the sheet
    for hr in historical_records {
        store.update_historical_record(&hr).await?;
    }

    println!("Historical data setup complete!");
    Ok(())
}

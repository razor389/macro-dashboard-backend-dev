// src/bin/setup_sheets.rs
use csv::Reader;
use dotenv::dotenv;
use std::{error::Error, fs::File};
use macro_dashboard_acm::services::sheets::{SheetsStore, SheetsConfig, HistoricalRecord};

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
    
    // Read CSV file
    let file = File::open("data/stk_mkt.csv")?;
    let mut rdr = Reader::from_reader(file);

    // Process and upload historical data
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
            eps: record[3].trim().parse().unwrap_or(0.0),
            cape: record[4].trim().parse()?,
        });
    }

    // Update records in batches to avoid API rate limits
    for record in historical_records {
        store.update_historical_record(&record).await?;
    }
    
    println!("Historical data setup complete!");
    Ok(())
}
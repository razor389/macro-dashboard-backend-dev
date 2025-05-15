use chrono::{Utc, Datelike};
use csv::Reader;
use log::{info, error};
use reqwest;
use std::error::Error as StdError;

pub type Result<T> = std::result::Result<T, Box<dyn StdError + Send + Sync>>;

/// Fetch the 4-week T-bill rate via the CSV endpoint
pub async fn fetch_tbill_data() -> Result<f64> {
    let year = Utc::now().year();
    let url = format!(
        "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/\
daily-treasury-rates.csv/{year}/all?_format=csv\
&field_tdr_date_value={year}\
&type=daily_treasury_bill_rates",
        year = year
    );
    info!("Fetching T-bill CSV from URL: {}", url);

    // Download & parse
    let csv_text = reqwest::get(&url).await?.text().await?;
    let mut rdr = Reader::from_reader(csv_text.as_bytes());

    // Locate the "4 Wk" column (4-week bill)
    let headers = rdr.headers()?.clone();
    let idx_4wk = headers
        .iter()
        .position(|h| h.trim() == "4 Wk")
        .ok_or("No '4 Wk' column in T-bill CSV")?;

    // Take the first data row (most recent date)
    if let Some(record) = rdr.records().next() {
        let row = record?;
        let cell = row
            .get(idx_4wk)
            .ok_or("Missing '4 Wk' field")?
            .trim();
        let rate = cell.parse::<f64>()?;
        info!("Found T-bill rate (4 Wk): {}", rate);
        return Ok(rate);
    }

    error!("No data rows in T-bill CSV");
    Err("No valid T-bill data found".into())
}

use chrono::{Utc, Datelike};
use csv::Reader;
use log::info;
use reqwest;
use std::error::Error;

/// Fetch the 20y nominal yield via the CSV endpoint
pub async fn fetch_20y_bond_yield() -> Result<f64, Box<dyn Error>> {
    let year = Utc::now().year();
    let url = format!(
        "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/\
daily-treasury-rates.csv/{year}/all?_format=csv\
&field_tdr_date_value={year}\
&type=daily_treasury_yield_curve",
        year = year
    );
    info!("Fetching 20-year bond yield CSV from URL: {}", url);

    // Download & parse
    let csv_text = reqwest::get(&url).await?.text().await?;
    let mut rdr = Reader::from_reader(csv_text.as_bytes());

    // Locate the "20 Yr" column
    let headers = rdr.headers()?.clone();
    let idx_20yr = headers
        .iter()
        .position(|h| h.trim() == "20 Yr")
        .ok_or("No '20 Yr' column in CSV")?;

    // Take the first data row (most recent date)
    if let Some(record) = rdr.records().next() {
        let row = record?;
        let cell = row
            .get(idx_20yr)
            .ok_or("Missing '20 Yr' field")?
            .trim();
        let rate = cell.parse::<f64>()?;
        info!("Found 20-year yield: {}", rate);
        return Ok(rate);
    }

    Err("No data rows in 20-year yield CSV".into())
}

/// Fetch the 20y TIPS yield via the CSV endpoint
pub async fn fetch_20y_tips_yield() -> Result<f64, Box<dyn Error>> {
    let year = Utc::now().year();
    let url = format!(
        "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/\
daily-treasury-rates.csv/{year}/all?_format=csv\
&field_tdr_date_value={year}\
&type=daily_treasury_real_yield_curve",
        year = year
    );
    info!("Fetching 20-year TIPS yield CSV from URL: {}", url);

    // Download & parse
    let csv_text = reqwest::get(&url).await?.text().await?;
    let mut rdr = Reader::from_reader(csv_text.as_bytes());

    // Locate the "20 Yr" column
    let headers = rdr.headers()?.clone();
    let idx_20yr = headers
        .iter()
        .position(|h| h.trim() == "20 Yr")
        .ok_or("No '20 Yr' column in TIPS CSV")?;

    // Take the first data row (most recent date)
    if let Some(record) = rdr.records().next() {
        let row = record?;
        let cell = row
            .get(idx_20yr)
            .ok_or("Missing '20 Yr' field in TIPS")?
            .trim();
        let rate = cell.parse::<f64>()?;
        info!("Found 20-year TIPS yield: {}", rate);
        return Ok(rate);
    }

    Err("No data rows in 20-year TIPS yield CSV".into())
}

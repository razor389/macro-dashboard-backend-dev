use chrono::{Utc, Datelike};
use csv::Reader;
use log::{info, warn, error};
use reqwest::Client;
use std::error::Error as StdError; // Using StdError for clarity
use std::time::Duration;

// Consistent Result type for functions in this module
type Result<T, E = Box<dyn StdError + Send + Sync>> = std::result::Result<T, E>;

// Internal helper function to fetch and parse a specific rate from a Treasury CSV URL
async fn fetch_treasury_csv_rate_generic(
    url: &str,
    column_name: &str,
    service_context: &str,
) -> Result<f64> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30)) // Add a reasonable timeout
        .build()?;

    info!("Fetching {} CSV from URL: {}", service_context, url);

    let response = client.get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .header("Accept", "text/csv,application/csv;q=0.9,*/*;q=0.8") // More specific for CSV
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Connection", "keep-alive") // Keep-alive can be useful
        .header("Sec-Fetch-Dest", "empty") // For direct resource fetch like CSV
        .header("Sec-Fetch-Mode", "cors")   // CSVs are often fetched cross-origin
        .header("Sec-Fetch-Site", "cross-site") // Assuming it's fetched from a different domain context
        .send()
        .await?;

    if !response.status().is_success() {
        let err_msg = format!(
            "Request for {} failed with status: {} for URL: {}",
            service_context, response.status(), url
        );
        error!("{}", err_msg);
        return Err(err_msg.into());
    }

    let csv_text = response.text().await?;
    if csv_text.trim().is_empty() {
        let err_msg = format!("Received empty CSV data for {} from URL: {}", service_context, url);
        warn!("{}", err_msg);
        return Err(err_msg.into());
    }

    let mut rdr = Reader::from_reader(csv_text.as_bytes());
    let headers = rdr.headers()?.clone();
    let col_idx = headers
        .iter()
        .position(|h| h.trim() == column_name)
        .ok_or_else(|| {
            let err_msg = format!(
                "No '{}' column in {} CSV from URL: {}. Headers found: {:?}",
                column_name, service_context, url, headers
            );
            error!("{}", err_msg);
            err_msg // Convert to Box<dyn Error> via .into() later
        })?;

    if let Some(record_result) = rdr.records().next() {
        let row = record_result?;
        let cell = row.get(col_idx)
            .ok_or_else(|| {
                format!(
                    "Column '{}' (index {}) missing in the first data row for {} CSV from URL: {}. Row: {:?}",
                    column_name, col_idx, service_context, url, row
                )
            })?
            .trim();

        if cell.eq_ignore_ascii_case("N/A") || cell.is_empty() {
            let err_msg = format!(
                "Data not available ('{}') for '{}' in {} CSV from URL: {}",
                cell, column_name, service_context, url
            );
            warn!("{}", err_msg);
            return Err(err_msg.into());
        }
        
        match cell.parse::<f64>() {
            Ok(rate) => {
                info!("Found {} ({}): {}", service_context, column_name, rate);
                Ok(rate)
            }
            Err(e) => {
                let err_msg = format!(
                    "Failed to parse rate '{}' for '{}' in {} CSV: {}. URL: {}",
                    cell, column_name, service_context, e, url
                );
                error!("{}", err_msg);
                Err(err_msg.into())
            }
        }
    } else {
        let err_msg = format!("No data records found in {} CSV from URL: {}", service_context, url);
        error!("{}", err_msg);
        Err(err_msg.into())
    }
}

/// Fetch the 20y nominal yield via the CSV endpoint
pub async fn fetch_20y_bond_yield() -> Result<f64> {
    let year = Utc::now().year();
    let url = format!(
        "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/\
daily-treasury-rates.csv/{year}/all?_format=csv\
&field_tdr_date_value={year}\
&type=daily_treasury_yield_curve",
        year = year
    );
    fetch_treasury_csv_rate_generic(&url, "20 Yr", "20-Year Nominal Bond Yield").await
}

/// Fetch the 20y TIPS yield via the CSV endpoint
pub async fn fetch_20y_tips_yield() -> Result<f64> {
    let year = Utc::now().year();
    let url = format!(
        "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/\
daily-treasury-rates.csv/{year}/all?_format=csv\
&field_tdr_date_value={year}\
&type=daily_treasury_real_yield_curve",
        year = year
    );
    fetch_treasury_csv_rate_generic(&url, "20 Yr", "20-Year TIPS Yield").await
}
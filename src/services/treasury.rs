use chrono::{Utc, Datelike};
use csv::Reader;
use log::{info, warn, error}; // Ensure warn is imported if used
use reqwest::Client; // Import Client
use std::error::Error as StdError;
use std::time::Duration;

// This type is already defined in your original code for this file.
pub type Result<T> = std::result::Result<T, Box<dyn StdError + Send + Sync>>;

// Internal helper function to fetch and parse a specific rate from a Treasury CSV URL
// Duplicated for modularity within this file, or could be moved to a shared treasury_common.rs
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
        .header("Connection", "keep-alive")
        .header("Sec-Fetch-Dest", "empty")
        .header("Sec-Fetch-Mode", "cors")
        .header("Sec-Fetch-Site", "cross-site")
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
        warn!("{}", err_msg); // Make sure `warn` is imported from `log`
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
            warn!("{}", err_msg); // Make sure `warn` is imported from `log`
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
    fetch_treasury_csv_rate_generic(&url, "4 Wk", "4-Week T-Bill Rate").await
}
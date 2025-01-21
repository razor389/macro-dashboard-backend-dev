use reqwest;
use scraper::{Html, Selector};
use log::{info, error};
use chrono::{Utc, Datelike};
use std::error::Error;

pub async fn fetch_tbill_data() -> Result<f64, Box<dyn Error>> {
    // Get the current year
    let current_year = Utc::now().year();
    let url = format!(
        "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/TextView?type=daily_treasury_bill_rates&field_tdr_date_value={}",
        current_year
    );
    info!("Fetching T-bill data from URL: {}", url);
    
    let response = reqwest::get(&url).await?;
    if !response.status().is_success() {
        error!("Failed to fetch the page. Status: {}", response.status());
        return Err(format!("Failed to fetch the page. Status: {}", response.status()).into());
    }
    
    let body = response.text().await?;
    info!("Received response body with length: {}", body.len());
    
    let document = Html::parse_document(&body);
    
    // Define selectors for table rows and the specific td element
    let row_selector = Selector::parse("table.views-table tbody tr").unwrap();
    let cell_selector = Selector::parse("td.views-field-field-br-round-b1-yield-4wk-2").unwrap();
    
    let mut last_valid_rate: Option<f64> = None;

    // Iterate over all rows, but we'll use the value from the last row
    for row in document.select(&row_selector) {
        if let Some(cell) = row.select(&cell_selector).next() {
            let tbill_rate_text = cell.text().collect::<String>().trim().to_string();
            info!("Extracted T-bill rate text: {}", tbill_rate_text);
            
            if tbill_rate_text != "N/A" && !tbill_rate_text.is_empty() {
                match tbill_rate_text.parse::<f64>() {
                    Ok(rate) => last_valid_rate = Some(rate),
                    Err(e) => error!("Failed to parse T-bill rate '{}' with error: {}", tbill_rate_text, e),
                }
            } else {
                info!("T-bill rate is 'N/A' or empty, continuing to next row.");
            }
        } else {
            info!("Desired cell not found in this row, continuing to next row.");
        }
    }
    
    // Return the last valid rate found
    if let Some(rate) = last_valid_rate {
        info!("Returning last valid T-bill rate: {}", rate);
        Ok(rate)
    } else {
        error!("No valid T-bill data found in the document.");
        Err("No valid T-bill data found.".into())
    }
}

use reqwest;
use scraper::{Html, Selector};
use log::{info, error};
use chrono::{Utc, Datelike};
use std::error::Error;

/// Fetch the 20y nominal yield by scraping the HTML
pub async fn fetch_20y_bond_yield() -> Result<f64, Box<dyn Error>> {
    let current_year = Utc::now().year();
    // This URL is similar to your T-Bill approach, but we use `daily_treasury_yield_curve`
    // instead of `daily_treasury_bill_rates`.
    let url = format!(
        "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/TextView?type=daily_treasury_yield_curve&field_tdr_date_value={}",
        current_year
    );
    info!("Fetching 20-year bond yield from URL: {}", url);

    let response = reqwest::get(&url).await?;
    if !response.status().is_success() {
        let err_msg = format!("Failed to fetch the page. Status: {}", response.status());
        error!("{}", err_msg);
        return Err(err_msg.into());
    }

    let body = response.text().await?;
    let document = Html::parse_document(&body);

    // You need to inspect the page's HTML to see the correct row/column
    // names for 20-year yields. The CSS classes can differ for each maturity.
    // Typically, these will look like "td.views-field-field-bond-yield-20-yr" or
    // "td.views-field-field-secn-benchmark-20y".
    //
    // The best way is to open your browser devtools, look at the actual <table> on
    // the Treasury site for the 20-year yield, and copy its class name.
    //
    // For example:
    let row_selector = Selector::parse("table.views-table tbody tr").unwrap();
    let cell_selector = Selector::parse("td.bc20year.views-field.views-field-field-bc-20year").unwrap();

    let mut last_valid_rate: Option<f64> = None;
    for row in document.select(&row_selector) {
        if let Some(cell) = row.select(&cell_selector).next() {
            let text = cell.text().collect::<String>().trim().to_string();
            if text != "N/A" && !text.is_empty() {
                if let Ok(parsed) = text.parse::<f64>() {
                    last_valid_rate = Some(parsed);
                }
            }
        }
    }

    match last_valid_rate {
        Some(rate) => {
            info!("Found 20-year yield: {}", rate);
            Ok(rate)
        }
        None => {
            let err_msg = "No valid 20-year bond yield found in the table.";
            error!("{}", err_msg);
            Err(err_msg.into())
        }
    }
}

/// Fetch the 20y TIPS yield similarly
pub async fn fetch_20y_tips_yield() -> Result<f64, Box<dyn Error>> {
    let current_year = Utc::now().year();
    // TIPS use `daily_treasury_real_yield_curve` in the `type=...` param
    let url = format!(
        "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/TextView?type=daily_treasury_real_yield_curve&field_tdr_date_value={}",
        current_year
    );
    info!("Fetching 20-year TIPS yield from URL: {}", url);

    let response = reqwest::get(&url).await?;
    if !response.status().is_success() {
        let err_msg = format!("Failed to fetch the page. Status: {}", response.status());
        error!("{}", err_msg);
        return Err(err_msg.into());
    }

    let body = response.text().await?;
    let document = Html::parse_document(&body);

    // Same approach, but the class name for the TIPS 20-year column is different.
    // E.g., "td.views-field-field-tips-bond-20-yr"
    // Inspect the HTML to find the right name.
    let row_selector = Selector::parse("table.views-table tbody tr").unwrap();
    let cell_selector = Selector::parse("td.tc20year.views-field.views-field-field-tc-10year").unwrap();

    let mut last_valid_rate: Option<f64> = None;
    for row in document.select(&row_selector) {
        if let Some(cell) = row.select(&cell_selector).next() {
            let text = cell.text().collect::<String>().trim().to_string();
            if text != "N/A" && !text.is_empty() {
                if let Ok(parsed) = text.parse::<f64>() {
                    last_valid_rate = Some(parsed);
                }
            }
        }
    }

    match last_valid_rate {
        Some(rate) => {
            info!("Found 20-year TIPS yield: {}", rate);
            Ok(rate)
        }
        None => {
            let err_msg = "No valid 20-year TIPS yield found in the table.";
            error!("{}", err_msg);
            Err(err_msg.into())
        }
    }
}

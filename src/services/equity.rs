//src/services/equity.rs
use reqwest::{self, Client};
use scraper::{Html, Selector};
use serde::Serialize;
use std::error::Error;
use log::info;
use regex::Regex;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use chrono::Datelike;

use crate::services::db::DbStore;

#[derive(Debug, Serialize)]
pub struct MarketData {
    pub sp500_price: f64,
    pub ttm_dividends: HashMap<String, f64>,
    pub eps_actual: HashMap<String, f64>,
    pub eps_estimated: HashMap<String, f64>,
    pub cape: f64,
    pub last_update: DateTime<Utc>
}

pub async fn get_market_data(db: &Arc<DbStore>) -> Result<MarketData, Box<dyn Error>> {
    let cache = db.get_market_cache().await?;
    
    Ok(MarketData {
        sp500_price: cache.sp500_price,
        ttm_dividends: cache.ttm_dividends,
        eps_actual: cache.eps_actual,
        eps_estimated: cache.eps_estimated,
        cape: cache.current_cape,
        last_update: cache.timestamps.ycharts_data,
    })
}

pub async fn fetch_sp500_price() -> Result<f64, Box<dyn Error>> {
    let url = "https://finance.yahoo.com/quote/%5EGSPC";
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()?;
        
    let resp = client.get(url).send().await?.text().await?;

    let re = Regex::new(r#"data-symbol="\^GSPC"[^>]*data-value="([0-9.]+)""#)?;
    let price = re.captures(&resp)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().parse::<f64>())
        .ok_or("Price not found")??;

    Ok(price)
}

#[derive(Debug)]
struct YChartsData {
    ttm_dividends: HashMap<String, f64>,
    eps_actual: HashMap<String, f64>,
    eps_estimated: HashMap<String, f64>,
    cape: f64,
}

fn parse_quarter_date(text: &str) -> Option<String> {
    let re_quarterly = Regex::new(r"Q(\d)\s+(\d{4})").ok()?;
    let re_monthly = Regex::new(r"([A-Za-z]+)\s+(\d{4})").ok()?;

    if let Some(caps) = re_quarterly.captures(text) {
        let quarter = caps.get(1)?.as_str();
        let year = caps.get(2)?.as_str();
        Some(format!("{}Q{}", year, quarter))
    } else if let Some(caps) = re_monthly.captures(text) {
        let month = match caps.get(1)?.as_str() {
            "Jan" | "Feb" | "Mar" => "1",
            "Apr" | "May" | "Jun" => "2",
            "Jul" | "Aug" | "Sep" => "3",
            "Oct" | "Nov" | "Dec" => "4",
            _ => return None,
        };
        let year = caps.get(2)?.as_str();
        Some(format!("{}Q{}", year, month))
    } else {
        None
    }
}

async fn fetch_ycharts_value(url: &str) -> Result<(String, f64), Box<dyn Error>> {
    info!("Fetching data from {}", url);
    
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await?
        .text()
        .await?;

    let document = Html::parse_document(&response);
    let value_selector = Selector::parse("div.key-stat-title").unwrap();
    
    let stat = document.select(&value_selector)
        .next()
        .and_then(|el| el.text().next())
        .ok_or_else(|| "Failed to find stat")?
        .trim();

    let re = Regex::new(r"(\d+\.?\d*)\s+(?:USD\s+)?(?:for\s+)?(?:Q\d\s+\d{4}|[A-Za-z]+\s+\d{4})")?;
    
    let value = re.captures(stat)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().parse::<f64>())
        .ok_or("Failed to parse value")??;

    let quarter = parse_quarter_date(stat)
        .ok_or("Failed to parse quarter")?;

    Ok((quarter, value))
}

async fn fetch_ycharts_data() -> Result<YChartsData, Box<dyn Error>> {
    let mut ttm_dividends = HashMap::new();
    let mut eps_actual = HashMap::new();
    let mut eps_estimated = HashMap::new();
    let mut cape = 0.0;

    // Fetch TTM Dividends
    if let Ok((quarter, value)) = fetch_ycharts_value(
        "https://ycharts.com/indicators/sp_500_dividends_per_share"
    ).await {
        ttm_dividends.insert(quarter, value);
    }

    // Fetch Current EPS
    if let Ok((quarter, value)) = fetch_ycharts_value(
        "https://ycharts.com/indicators/sp_500_eps"
    ).await {
        eps_actual.insert(quarter, value);
    }

    // Fetch Forward EPS
    if let Ok((quarter, value)) = fetch_ycharts_value(
        "https://ycharts.com/indicators/sp_500_earnings_per_share_forward_estimate"
    ).await {
        eps_estimated.insert(quarter, value);
    }

    // Fetch CAPE
    if let Ok((_, value)) = fetch_ycharts_value(
        "https://ycharts.com/indicators/cyclically_adjusted_pe_ratio"
    ).await {
        cape = value;
    }

    Ok(YChartsData {
        ttm_dividends,
        eps_actual,
        eps_estimated,
        cape,
    })
}

pub async fn update_market_data(db: &Arc<DbStore>) -> Result<(), Box<dyn Error>> {
    let mut cache = db.get_market_cache().await?;
    
    if cache.needs_yahoo_update() {
        info!("Fetching updated S&P 500 price");
        if let Ok(price) = fetch_sp500_price().await {
            cache.sp500_price = price;
            cache.timestamps.yahoo_price = Utc::now();
        }
    }

    if cache.needs_ycharts_update() {
        info!("Fetching updated YCharts data");
        if let Ok(ycharts_data) = fetch_ycharts_data().await {
            // Update dividends
            for (quarter, value) in ycharts_data.ttm_dividends {
                cache.ttm_dividends.insert(quarter, value);
            }
            
            // Update EPS data
            for (quarter, value) in ycharts_data.eps_actual {
                cache.eps_actual.insert(quarter, value);
            }
            for (quarter, value) in ycharts_data.eps_estimated {
                cache.eps_estimated.insert(quarter, value);
            }
            
            cache.current_cape = ycharts_data.cape;
            cache.timestamps.ycharts_data = Utc::now();
        }
    }

    db.update_market_cache(&cache).await?;
    Ok(())
}

// Function to analyze completed years and update historical data
pub async fn analyze_complete_years(db: &Arc<DbStore>) -> Result<(), Box<dyn Error>> {
    let cache = db.get_market_cache().await?;
    let historical = db.get_historical_data().await?;
    
    // Get the latest year in historical data
    let latest_year = historical.iter().map(|r| r.year).max().unwrap_or(0);
    let current_year = Utc::now().year() as i32 - 1;
    
    if current_year > latest_year {
        // Check if we have complete data for the year
        let mut quarterly_eps = Vec::new();
        let mut quarterly_div = Vec::new();
        
        for quarter in 1..=4 {
            let q = format!("{}Q{}", current_year, quarter);
            if let Some(eps) = cache.eps_actual.get(&q) {
                quarterly_eps.push(*eps);
            }
            if let Some(div) = cache.ttm_dividends.get(&q) {
                quarterly_div.push(*div);
            }
        }
        
        // If we have complete quarterly data
        if quarterly_eps.len() == 4 && quarterly_div.len() == 4 {
            let total_eps: f64 = quarterly_eps.iter().sum();
            let avg_div: f64 = quarterly_div.iter().sum::<f64>() / 4.0;
            
            // Create historical record
            let record = crate::services::db::HistoricalRecord {
                year: current_year,
                sp500_price: cache.sp500_price,
                dividend: avg_div,
                eps: total_eps,
                cape: cache.current_cape,
            };
            
            db.add_historical_data(record).await?;
        }
    }
    
    Ok(())
}
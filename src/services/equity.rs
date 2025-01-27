//src/services/equity.rs
use reqwest::{self, Client};
use scraper::{Html, Selector};
use serde::Serialize;
use std::error::Error;
use log::{error, info};
use regex::Regex;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use chrono::Datelike;
use bigdecimal::BigDecimal;
use std::str::FromStr;

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
    info!("Fetching data from URL: {}", url);
    
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
    
    info!("Found stat text: {}", stat);

    let re = Regex::new(r"([-+]?\d*\.?\d+)\s*(?:USD)?\s*(?:for\s+)?(?:Q\d\s+\d{4}|[A-Za-z]+\s+\d{4})")?;
    
    let (value, quarter_text) = match re.captures(stat) {
        Some(caps) => {
            let value_str = caps.get(1).ok_or("No value match")?.as_str();
            let full_match = caps.get(0).ok_or("No full match")?.as_str();
            let quarter_part = full_match.split_whitespace().skip(1).collect::<Vec<_>>().join(" ");
            info!("Parsed value: {}, quarter text: {}", value_str, quarter_part);
            (value_str.parse::<f64>()?, quarter_part)
        },
        None => {
            error!("Failed to parse value and quarter from stat: {}", stat);
            return Err("Failed to parse value and quarter".into())
        }
    };

    let quarter = match parse_quarter_date(&quarter_text) {
        Some(q) => {
            info!("Successfully parsed quarter: {}", q);
            q
        },
        None => {
            error!("Failed to parse quarter from text: {}", quarter_text);
            return Err("Failed to parse quarter".into())
        }
    };

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
    info!("Starting market data update");
    let mut cache = db.get_market_cache().await?;
    let mut data_updated = false;
    
    if cache.needs_yahoo_update() {
        info!("Fetching updated S&P 500 price");
        if let Ok(price) = fetch_sp500_price().await {
            cache.sp500_price = price;
            cache.timestamps.yahoo_price = Utc::now();
            data_updated = true;
        }
    }

    if cache.needs_ycharts_update() {
        info!("Fetching updated YCharts data");
        if let Ok(ycharts_data) = fetch_ycharts_data().await {
            // Update dividends
            for (quarter, value) in ycharts_data.ttm_dividends {
                cache.ttm_dividends.insert(quarter.clone(), value);
                info!("Updated TTM dividend for {}: {}", quarter, value);
            }
            
            // Update EPS data and ensure data is merged, not replaced
            for (quarter, value) in ycharts_data.eps_actual {
                cache.eps_actual.insert(quarter.clone(), value);
                info!("Updated actual EPS for {}: {}", quarter, value);
            }
            
            for (quarter, value) in ycharts_data.eps_estimated {
                cache.eps_estimated.insert(quarter.clone(), value);
                info!("Updated estimated EPS for {}: {}", quarter, value);
            }
            
            cache.current_cape = ycharts_data.cape;
            cache.timestamps.ycharts_data = Utc::now();
            data_updated = true;
        }
    }

    if data_updated {
        info!("Updating market cache in database");
        
        // First update the market_cache table
        sqlx::query!(
            r#"
            INSERT INTO market_cache (sp500_price, current_cape, last_yahoo_update, last_ycharts_update)
            VALUES ($1, $2, $3, $4)
            "#,
            cache.sp500_price as f64,
            cache.current_cape as f64,
            cache.timestamps.yahoo_price,
            cache.timestamps.ycharts_data,
        )
        .execute(&db.pool)
        .await?;

        // Then update quarterly_data table for each data point
        for (quarter, dividend) in &cache.ttm_dividends {
            let eps_actual = cache.eps_actual.get(quarter);
            let eps_estimated = cache.eps_estimated.get(quarter);
            
            sqlx::query!(
                r#"
                INSERT INTO quarterly_data (quarter, ttm_dividend, eps_actual, eps_estimated, updated_at)
                VALUES ($1, $2, $3, $4, NOW())
                ON CONFLICT (quarter) 
                DO UPDATE SET 
                    ttm_dividend = EXCLUDED.ttm_dividend,
                    eps_actual = CASE 
                        WHEN EXCLUDED.eps_actual IS NOT NULL THEN EXCLUDED.eps_actual 
                        ELSE quarterly_data.eps_actual 
                    END,
                    eps_estimated = CASE 
                        WHEN EXCLUDED.eps_estimated IS NOT NULL THEN EXCLUDED.eps_estimated 
                        ELSE quarterly_data.eps_estimated 
                    END,
                    updated_at = NOW()
                "#,
                quarter,
                BigDecimal::from_str(&dividend.to_string())?,
                eps_actual.map(|v| BigDecimal::from_str(&v.to_string())).transpose()?,
                eps_estimated.map(|v| BigDecimal::from_str(&v.to_string())).transpose()?,
            )
            .execute(&db.pool)
            .await?;
        }

        // Additional separate inserts for EPS data without dividends
        for (quarter, value) in &cache.eps_actual {
            if !cache.ttm_dividends.contains_key(quarter) {
                sqlx::query!(
                    r#"
                    INSERT INTO quarterly_data (quarter, eps_actual, updated_at)
                    VALUES ($1, $2, NOW())
                    ON CONFLICT (quarter) 
                    DO UPDATE SET 
                        eps_actual = EXCLUDED.eps_actual,
                        updated_at = NOW()
                    "#,
                    quarter,
                    BigDecimal::from_str(&value.to_string())?,
                )
                .execute(&db.pool)
                .await?;
            }
        }

        for (quarter, value) in &cache.eps_estimated {
            if !cache.ttm_dividends.contains_key(quarter) {
                sqlx::query!(
                    r#"
                    INSERT INTO quarterly_data (quarter, eps_estimated, updated_at)
                    VALUES ($1, $2, NOW())
                    ON CONFLICT (quarter) 
                    DO UPDATE SET 
                        eps_estimated = EXCLUDED.eps_estimated,
                        updated_at = NOW()
                    "#,
                    quarter,
                    BigDecimal::from_str(&value.to_string())?,
                )
                .execute(&db.pool)
                .await?;
            }
        }

        // After updating the database, check if we have Q4 data for previous year
        let prev_year = Utc::now().year() - 1;
        let q4_key = format!("{}Q4", prev_year);
        
        if cache.eps_actual.contains_key(&q4_key) {
            info!("Found Q4 data for year {}, analyzing complete year", prev_year);
            analyze_complete_years(db).await?;
        }
    }

    Ok(())
}

pub async fn analyze_complete_years(db: &Arc<DbStore>) -> Result<(), Box<dyn Error>> {
    info!("Starting complete year analysis");
    let cache = db.get_market_cache().await?;
    let historical = db.get_historical_data().await?;
    
    let latest_year = historical.iter().map(|r| r.year).max().unwrap_or(0);
    let current_year = Utc::now().year() as i32 - 1;
    
    if current_year > latest_year {
        info!("Analyzing year {} for historical data", current_year);
        
        let mut quarterly_eps = Vec::new();
        let mut quarterly_div = Vec::new();
        
        // Check each quarter
        for quarter in 1..=4 {
            let q = format!("{}Q{}", current_year, quarter);
            if let Some(eps) = cache.eps_actual.get(&q) {
                info!("Found EPS for {}: {}", q, eps);
                quarterly_eps.push(*eps);
            } else {
                info!("Missing EPS for quarter {}", q);
                return Ok(()); // Exit if we don't have complete data
            }
            
            if let Some(div) = cache.ttm_dividends.get(&q) {
                info!("Found dividend for {}: {}", q, div);
                quarterly_div.push(*div);
            } else {
                info!("Missing dividend for quarter {}", q);
                return Ok(()); // Exit if we don't have complete data
            }
        }
        
        // If we have complete quarterly data
        if quarterly_eps.len() == 4 && quarterly_div.len() == 4 {
            let total_eps: f64 = quarterly_eps.iter().sum();
            let avg_div: f64 = quarterly_div.iter().sum::<f64>() / 4.0;
            
            info!("Creating historical record for year {}", current_year);
            info!("Total EPS: {}, Average Dividend: {}", total_eps, avg_div);
            
            let record = crate::services::db::HistoricalRecord {
                year: current_year,
                sp500_price: cache.sp500_price,
                dividend: avg_div,
                eps: total_eps,
                cape: cache.current_cape,
            };
            
            db.add_historical_data(record).await?;
            info!("Successfully added historical record for year {}", current_year);
        }
    } else {
        info!("No new complete years to analyze after year {}", latest_year);
    }
    
    Ok(())
}

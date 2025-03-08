//src/services/equity.rs
use reqwest::{self, Client};
use scraper::{Html, Selector};
use serde::Serialize;
use log::{error,info};
use regex::Regex;
use chrono::{DateTime, Utc, NaiveTime, Datelike, Duration};
use std::collections::HashMap;
use std::sync::Arc;
use chrono_tz::US::Central;
use anyhow::Result;

use crate::models::{HistoricalRecord, MonthlyData, QuarterlyData};

use super::{calculations::{calculate_market_metrics, MarketMetrics}, db::DbStore};

#[derive(Debug, Serialize)]
pub struct QuarterlyValue {
    pub final_quarter: String,
    pub value: f64,
}

#[derive(Debug, Serialize)]
pub struct MarketData {
    pub daily_close_sp500_price: f64,
    pub current_sp500_price: f64,
    pub ttm_dividend: Option<QuarterlyValue>,
    pub latest_eps_actual: Option<QuarterlyValue>,
    pub estimated_eps_sum: Option<QuarterlyValue>,
    pub cape: f64,
    pub cape_period: String,
    pub last_update: DateTime<Utc>
}

#[derive(Debug)]
struct YChartsData {
    quarterly_dividends: HashMap<String, f64>,
    eps_actual: HashMap<String, f64>,
    eps_estimated: HashMap<String, f64>,
    cape: (f64, String), // (value, period)
    monthly_return: Option<(String, f64)>, // (period, value)
}

async fn get_quarterly_calculations(db: &Arc<DbStore>) -> Result<(Option<QuarterlyValue>, Option<QuarterlyValue>, Option<QuarterlyValue>)> {
    let quarterly_data = db.sheets_store.get_quarterly_data().await?;
    
    // Sort quarters in descending order (most recent first)
    let mut sorted_data = quarterly_data.clone();
    sorted_data.sort_by(|a, b| {
        let parse_quarter = |q: &str| {
            let year: i32 = q[..4].parse().unwrap_or(0);
            let quarter: i32 = q[5..].parse().unwrap_or(0);
            (year, quarter)
        };
        let (year_b, q_b) = parse_quarter(&b.quarter);
        let (year_a, q_a) = parse_quarter(&a.quarter);
        (year_a, q_a).cmp(&(year_b, q_b))
    });

    // Calculate TTM dividend (sum of most recent 4 quarters)
    let ttm_dividend = {
        let mut quarters_found = 0;
        let mut sum = 0.0;
        let mut final_quarter = None;

        for record in sorted_data.iter().rev() {
            if let Some(dividend) = record.dividend {
                if quarters_found == 0 {
                    final_quarter = Some(record.quarter.clone());
                }
                sum += dividend;
                quarters_found += 1;
                if quarters_found == 4 {
                    break;
                }
            }
        }

        if quarters_found == 4 {
            Some(QuarterlyValue {
                final_quarter: final_quarter.unwrap(),
                value: sum,
            })
        } else {
            None
        }
    };

    // Get latest actual EPS
    let latest_eps_actual = sorted_data.iter().rev()
        .find(|q| q.eps_actual.is_some())
        .map(|q| QuarterlyValue {
            final_quarter: q.quarter.clone(),
            value: q.eps_actual.unwrap()
        });

    // Calculate sum of next 4 quarters of estimated EPS
    let estimated_eps_sum = {
        let mut quarters_found = 0;
        let mut sum = 0.0;
        let mut final_quarter = None;

        // Find first quarter with estimate
        if let Some(start_idx) = sorted_data.iter()
            .position(|q| q.eps_estimated.is_some()) {
                
            let mut consecutive_quarters = true;
            let mut current_idx = start_idx;
            
            while current_idx < sorted_data.len() && quarters_found < 4 {
                if let Some(eps) = sorted_data[current_idx].eps_estimated {
                    if quarters_found == 0 {
                        final_quarter = Some(sorted_data[current_idx + 3].quarter.clone());
                    }
                    sum += eps;
                    quarters_found += 1;
                } else {
                    consecutive_quarters = false;
                    break;
                }
                current_idx += 1;
            }

            if quarters_found == 4 && consecutive_quarters {
                Some(QuarterlyValue {
                    final_quarter: final_quarter.unwrap(),
                    value: sum,
                })
            } else {
                None
            }
        } else {
            None
        }
    };

    Ok((ttm_dividend, latest_eps_actual, estimated_eps_sum))
}

pub async fn get_market_data(db: &Arc<DbStore>) -> Result<MarketData> {
    let mut cache = db.get_market_cache().await?;
    let mut data_updated = false;

    // Existing price update logic...
    if cache.current_sp500_price == 0.0 {
        info!("Initial fetch of current S&P 500 price");
        if let Ok(price) = fetch_sp500_price().await {
            cache.current_sp500_price = price;
            cache.timestamps.yahoo_price = Utc::now();
            data_updated = true;
        }
    }

    if cache.timestamps.yahoo_price < Utc::now() - Duration::minutes(15) {
        info!("Updating current S&P 500 price (15-minute interval)");
        if let Ok(price) = fetch_sp500_price().await {
            cache.current_sp500_price = price;
            cache.timestamps.yahoo_price = Utc::now();
            data_updated = true;
        }
    }

    if should_update_daily() {
        info!("Market close time - performing daily updates");
        if let Ok(price) = fetch_sp500_price().await {
            cache.daily_close_sp500_price = price;
            cache.current_sp500_price = price;
            data_updated = true;
        }

        if let Ok(ycharts_data) = fetch_ycharts_data().await {
            // Check if we got a new monthly return
            if let Some((month, return_value)) = &ycharts_data.monthly_return {
                // Update the monthly data sheet if it's a new month
                if let Err(e) = update_monthly_data(db, month, *return_value).await {
                    error!("Failed to update monthly data sheet: {}", e);
                }
            }
            
            // Update quarterly dividend data
            if !ycharts_data.quarterly_dividends.is_empty() {
                if let Err(e) = update_quarterly_data(db, &ycharts_data.quarterly_dividends, "dividend").await {
                    error!("Failed to update quarterly dividend data: {}", e);
                }
            }
            
            // Update quarterly EPS actual data
            if !ycharts_data.eps_actual.is_empty() {
                if let Err(e) = update_quarterly_data(db, &ycharts_data.eps_actual, "eps_actual").await {
                    error!("Failed to update quarterly EPS actual data: {}", e);
                }
            }
            
            // Update quarterly EPS estimated data
            if !ycharts_data.eps_estimated.is_empty() {
                if let Err(e) = update_quarterly_data(db, &ycharts_data.eps_estimated, "eps_estimated").await {
                    error!("Failed to update quarterly EPS estimated data: {}", e);
                }
            }
            
            update_cache_from_ycharts(&mut cache, ycharts_data);
            cache.timestamps.ycharts_data = Utc::now();
            data_updated = true;
        }
    }

    if data_updated {
        info!("Cache updated");
        db.update_market_cache(&cache).await?;
        check_historical_updates(db, &cache).await?;
    }

    // Get latest quarterly data
    let (ttm_dividend, latest_eps_actual, estimated_eps_sum) = get_quarterly_calculations(db).await?;
    
    Ok(MarketData {
        daily_close_sp500_price: cache.daily_close_sp500_price,
        current_sp500_price: cache.current_sp500_price,
        ttm_dividend,
        latest_eps_actual,
        estimated_eps_sum,
        cape: cache.current_cape,
        cape_period: cache.cape_period.clone(),
        last_update: cache.timestamps.ycharts_data,
    })
}

fn should_update_daily() -> bool {
    let current_ct = Utc::now().with_timezone(&Central);
    let target_time = NaiveTime::from_hms_opt(15, 30, 0).unwrap();
    let current_time = current_ct.time();
    current_time >= target_time && 
    current_time < target_time + chrono::Duration::minutes(1)
}

async fn fetch_sp500_price() -> Result<f64> {
    let url = "https://finance.yahoo.com/quote/%5EGSPC";
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()?;
        
    let resp = client.get(url).send().await?.text().await?;
    let re = Regex::new(r#"data-symbol="\^GSPC"[^>]*data-value="([0-9.]+)""#)?;
    let price = re.captures(&resp)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().parse::<f64>())
        .ok_or_else(|| anyhow::anyhow!("Price not found"))??;

    Ok(price)
}

async fn fetch_ycharts_value(url: &str) -> Result<(String, f64)> {
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
        .ok_or_else(||anyhow::anyhow!("Failed to find stat"))?
        .trim();
    
    info!("Found stat text: {}", stat);

    // IMPROVED REGEX - handles the current YCharts format better
    let re = Regex::new(r"([-+]?\d*\.?\d+)%?\s*(?:USD)?\s*(?:for)?\s+(?:(Q\d)\s+(\d{4})|(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{4}))")?;
    
    if let Some(caps) = re.captures(stat) {
        let value_str = caps.get(1).ok_or(anyhow::anyhow!("No value match"))?.as_str();
        let value = value_str.parse::<f64>()?;
        
        let period_text = if let Some(quarter) = caps.get(2) {
            // It's quarterly data: Q1 2024 format
            let year = caps.get(3).unwrap().as_str();
            format!("{}{}", year, quarter.as_str())
        } else {
            // It's monthly data: Jan 2024 format
            let month = caps.get(4).unwrap().as_str();
            let year = caps.get(5).unwrap().as_str();
            
            // Convert month name to number
            let month_num = match month {
                "Jan" => "01", "Feb" => "02", "Mar" => "03", "Apr" => "04",
                "May" => "05", "Jun" => "06", "Jul" => "07", "Aug" => "08",
                "Sep" => "09", "Oct" => "10", "Nov" => "11", "Dec" => "12",
                _ => "00" // shouldn't happen with the regex
            };
            
            // Format as YYYY-MM for consistent sorting
            format!("{}-{}", year, month_num)
        };
        
        // Convert percentage to decimal if needed
        let final_value = if stat.contains('%') {
            value / 100.0
        } else {
            value
        };
        
        return Ok((period_text, final_value));
    }
    
    // If regex didn't match, try a simpler approach to at least extract the value
    let fallback_re = Regex::new(r"([-+]?\d*\.?\d+)%?")?;
    if let Some(caps) = fallback_re.captures(stat) {
        let value_str = caps.get(1).ok_or(anyhow::anyhow!("No value match with fallback"))?.as_str();
        let value = value_str.parse::<f64>()?;
        let final_value = if stat.contains('%') {
            value / 100.0
        } else {
            value
        };
        
        // Try to extract period from text
        let year_re = Regex::new(r"\b(20\d{2})\b").unwrap();
        let period = if let Some(year_caps) = year_re.captures(stat) {
            let year = year_caps.get(1).unwrap().as_str();
            
            // Look for month or quarter
            let month_re = Regex::new(r"\b(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\b").unwrap();
            if let Some(month_caps) = month_re.captures(stat) {
                let month = month_caps.get(1).unwrap().as_str();
                let month_num = match month {
                    "Jan" => "01", "Feb" => "02", "Mar" => "03", "Apr" => "04",
                    "May" => "05", "Jun" => "06", "Jul" => "07", "Aug" => "08", 
                    "Sep" => "09", "Oct" => "10", "Nov" => "11", "Dec" => "12",
                    _ => "00"
                };
                format!("{}-{}", year, month_num)
            } else {
                let quarter_re = Regex::new(r"\b(Q[1-4])\b").unwrap();
                if let Some(q_caps) = quarter_re.captures(stat) {
                    let quarter = q_caps.get(1).unwrap().as_str();
                    format!("{}{}", year, quarter)
                } else {
                    format!("{}-00", year)
                }
            }
        } else {
            "Unknown".to_string()
        };
        
        return Ok((period, final_value));
    }
    
    Err(anyhow::anyhow!("Failed to parse value and period"))
}

async fn fetch_ycharts_data() -> Result<YChartsData> {
    let mut quarterly_dividends = HashMap::new();
    let mut eps_actual = HashMap::new();
    let mut eps_estimated = HashMap::new();
    let mut cape = (0.0, String::new());
    let mut monthly_return = None;

    // Fetch quarterly dividend
    if let Ok((quarter, value)) = fetch_ycharts_value(
        "https://ycharts.com/indicators/sp_500_dividends_per_share"
    ).await {
        quarterly_dividends.insert(quarter, value);
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

    // Fetch CAPE with period
    if let Ok((period, value)) = fetch_ycharts_value(
        "https://ycharts.com/indicators/cyclically_adjusted_pe_ratio"
    ).await {
        cape = (value, period);
    }

    // Fetch monthly return
    if let Ok((period, value)) = fetch_ycharts_value(
        "https://ycharts.com/indicators/sp_500_monthly_total_return"
    ).await {
        // Convert percentage to decimal
        monthly_return = Some((period, value / 100.0));
    }

    Ok(YChartsData {
        quarterly_dividends,
        eps_actual,
        eps_estimated,
        cape,
        monthly_return,
    })
}

fn update_cache_from_ycharts(cache: &mut crate::models::MarketCache, ycharts_data: YChartsData) {
    // Update quarterly dividends
    for (quarter, value) in ycharts_data.quarterly_dividends {
        cache.quarterly_dividends.insert(quarter, value);
    }
    
    // Update EPS data
    for (quarter, value) in ycharts_data.eps_actual {
        cache.eps_actual.insert(quarter, value);
    }
    
    for (quarter, value) in ycharts_data.eps_estimated {
        cache.eps_estimated.insert(quarter, value);
    }

    if let Some((month, return_value)) = ycharts_data.monthly_return {
        cache.latest_month = month;
        cache.latest_monthly_return = return_value;
    }
    
    cache.current_cape = ycharts_data.cape.0;
    cache.cape_period = ycharts_data.cape.1;
}

pub async fn update_monthly_data(db: &Arc<DbStore>, month: &str, return_value: f64) ->  Result<()> {
    info!("Updating monthly data for {}: {}", month, return_value);
    
    // Get existing monthly data
    let mut monthly_data = db.sheets_store.get_monthly_data().await?;
    
    // Check if this month already exists
    let month_exists = monthly_data.iter().any(|data| data.month == month);
    
    if !month_exists {
        info!("Adding new month data: {} = {}", month, return_value);
        // Add the new month
        monthly_data.push(MonthlyData {
            month: month.to_string(),
            total_return: return_value,
        });
        
        // Sort monthly data by date for consistency
        monthly_data.sort_by(|a, b| a.month.cmp(&b.month));
        
        // Update the sheet
        db.sheets_store.update_monthly_data(&monthly_data).await?;
        info!("Successfully updated monthly data sheet with new month: {}", month);
    } else {
        info!("Month {} already exists in monthly data, skipping update", month);
    }
    
    Ok(())
}

pub async fn update_quarterly_data(db: &Arc<DbStore>, quarterly_data: &HashMap<String, f64>, data_type: &str) ->  Result<()> {
    if quarterly_data.is_empty() {
        info!("No quarterly {} data to update", data_type);
        return Ok(());
    }

    info!("Updating quarterly {} data with {} entries", data_type, quarterly_data.len());
    
    // Get existing quarterly data
    let mut existing_data = db.sheets_store.get_quarterly_data().await?;
    info!("Retrieved {} existing quarterly records", existing_data.len());
    
    let mut updates_made = false;
    
    // Update existing or add new quarterly data
    for (quarter, value) in quarterly_data {
        // Find existing entry for this quarter
        let existing_entry = existing_data.iter_mut().find(|entry| &entry.quarter == quarter);
        
        match existing_entry {
            Some(entry) => {
                // Update the appropriate field based on data type
                match data_type {
                    "dividend" => {
                        if entry.dividend.is_none() || (entry.dividend.unwrap() - *value).abs() > 0.001 {
                            info!("Updating dividend for {} from {:?} to {}", 
                                  quarter, entry.dividend, value);
                            entry.dividend = Some(*value);
                            updates_made = true;
                        }
                    },
                    "eps_actual" => {
                        if entry.eps_actual.is_none() || (entry.eps_actual.unwrap() - *value).abs() > 0.001 {
                            info!("Updating EPS actual for {} from {:?} to {}", 
                                  quarter, entry.eps_actual, value);
                            entry.eps_actual = Some(*value);
                            updates_made = true;
                        }
                    },
                    "eps_estimated" => {
                        if entry.eps_estimated.is_none() || (entry.eps_estimated.unwrap() - *value).abs() > 0.001 {
                            info!("Updating EPS estimate for {} from {:?} to {}", 
                                  quarter, entry.eps_estimated, value);
                            entry.eps_estimated = Some(*value);
                            updates_made = true;
                        }
                    },
                    _ => {
                        error!("Unknown data type: {}", data_type);
                    }
                }
            },
            None => {
                // Create a new entry for this quarter
                info!("Adding new {} of {} for quarter {}", data_type, value, quarter);
                
                let mut new_entry = QuarterlyData {
                    quarter: quarter.clone(),
                    dividend: None,
                    eps_actual: None,
                    eps_estimated: None,
                };
                
                // Set the appropriate field based on data type
                match data_type {
                    "dividend" => new_entry.dividend = Some(*value),
                    "eps_actual" => new_entry.eps_actual = Some(*value),
                    "eps_estimated" => new_entry.eps_estimated = Some(*value),
                    _ => {
                        error!("Unknown data type: {}", data_type);
                    }
                }
                
                existing_data.push(new_entry);
                updates_made = true;
            }
        }
    }
    
    // If we made any updates, save the data back to the sheet
    if updates_made {
        info!("Saving updated quarterly data to sheet");
        
        // Sort the data by quarter for consistency
        existing_data.sort_by(|a, b| {
            // Parse quarters like "2024Q1" for proper sorting
            let parse_quarter = |q: &str| -> (i32, i32) {
                let year = q.get(0..4).unwrap_or("0000").parse::<i32>().unwrap_or(0);
                let quarter = q.get(4..6).unwrap_or("0").parse::<i32>().unwrap_or(0);
                (year, quarter)
            };
            
            let a_parts = parse_quarter(&a.quarter);
            let b_parts = parse_quarter(&b.quarter);
            a_parts.cmp(&b_parts)
        });
        
        db.sheets_store.update_quarterly_data(&existing_data).await?;
        info!("Quarterly data successfully updated");
    } else {
        info!("No updates needed for quarterly data");
    }
    
    Ok(())
}

async fn check_historical_updates(db: &Arc<DbStore>, cache: &crate::models::MarketCache) -> Result<()> {
    let current_year = Utc::now().year() as i32;
    let prev_year = current_year - 1;
    
    // Get existing record or create new one
    let mut historical_record = match db.get_historical_year(prev_year).await? {
        Some(record) => record,
        None => HistoricalRecord {
            year: prev_year,
            sp500_price: 0.0,
            dividend: 0.0,
            dividend_yield: 0.0,
            eps: 0.0,
            cape: 0.0,
            inflation: 0.0,
            total_return: 0.0,
            cumulative_return: 0.0
        }
    };
    
    let mut updates_needed = false;

    // Check if we have new Q4 data to update previous year
    let q4_key = format!("{}Q4", prev_year);
    
    if cache.eps_actual.contains_key(&q4_key) || cache.quarterly_dividends.contains_key(&q4_key) {
        let mut eps_sum = 0.0;
        let mut div_sum = 0.0;
        let mut have_complete_eps = true;
        let mut have_complete_div = true;

        // Sum up quarterly values
        for quarter in 1..=4 {
            let q = format!("{}Q{}", prev_year, quarter);
            
            if let Some(eps) = cache.eps_actual.get(&q) {
                eps_sum += eps;
            } else {
                have_complete_eps = false;
            }
            
            if let Some(div) = cache.quarterly_dividends.get(&q) {
                div_sum += div;
            } else {
                have_complete_div = false;
            }
        }

        if have_complete_eps {
            historical_record.eps = eps_sum;
            updates_needed = true;
            info!("Updated historical EPS for {}: {}", prev_year, eps_sum);
        }
        
        if have_complete_div {
            historical_record.dividend = div_sum;
            updates_needed = true;
            info!("Updated historical dividend for {}: {}", prev_year, div_sum);
        }
    }

    // Check for year change since last Yahoo price update
    let last_update = cache.timestamps.yahoo_price.naive_utc().date();
    let current_date = Utc::now().naive_utc().date();
    
    if last_update.year() < current_date.year() && cache.daily_close_sp500_price > 0.0 {
        historical_record.sp500_price = cache.daily_close_sp500_price;
        updates_needed = true;
        info!("Updated historical closing price for {} based on year change: {}", 
              prev_year, cache.daily_close_sp500_price);
    }

    // Check if we have complete monthly data for the previous year
    let monthly_data = db.sheets_store.get_monthly_data().await?;
    if let Some(yearly_return) = compute_yearly_return(&monthly_data, prev_year) {
        historical_record.total_return = yearly_return;
        updates_needed = true;
        info!("Updated historical total return for {}: {}", prev_year, yearly_return);
    }

    // Check if we have a December CAPE value
    if cache.cape_period == format!("Dec {}", prev_year) {
        historical_record.cape = cache.current_cape;
        updates_needed = true;
        info!("Updated historical CAPE for {}: {}", prev_year, cache.current_cape);
    }

    if updates_needed {
        if historical_record.sp500_price > 0.0 && historical_record.dividend > 0.0 {
            historical_record.dividend_yield = historical_record.dividend / historical_record.sp500_price;
        }
        
        db.update_historical_record(historical_record).await?;
        info!("Successfully updated historical record for {}", prev_year);
    }

    Ok(())
}

pub async fn get_market_metrics(db: &Arc<DbStore>) -> Result<MarketMetrics> {
    let historical_data = db.get_historical_data().await?;
    calculate_market_metrics(&historical_data)
}

pub async fn get_historical_data(db: &Arc<DbStore>) -> Result<Vec<HistoricalRecord>> {
    db.get_historical_data().await
}

pub async fn get_historical_data_range(
    db: &Arc<DbStore>, 
    start_year: i32, 
    end_year: i32
) -> Result<Vec<HistoricalRecord>> {
    let all_data = db.get_historical_data().await?;
    Ok(all_data.into_iter()
        .filter(|record| record.year >= start_year && record.year <= end_year)
        .collect())
}

fn compute_yearly_return(monthly_data: &[MonthlyData], year: i32) -> Option<f64> {
    let year_prefix = format!("{}-", year);
    let year_returns: Vec<f64> = monthly_data.iter()
        .filter(|data| data.month.starts_with(&year_prefix))
        .map(|data| data.total_return)
        .collect();

    if year_returns.len() == 12 {
        // Compute compound return: (1 + r1) * (1 + r2) * ... * (1 + r12) - 1
        Some(year_returns.iter()
            .fold(1.0, |acc, &r| acc * (1.0 + r)) - 1.0)
    } else {
        None
    }
}
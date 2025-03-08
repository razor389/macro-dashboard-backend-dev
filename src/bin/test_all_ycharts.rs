// src/bin/test_all_ycharts.rs
// Run with: cargo run --bin test_all_ycharts

use dotenv::dotenv;
use env_logger;
use log::{info, error};
use std::error::Error;
use scraper::{Html, Selector};
use reqwest::Client;
use regex::Regex;

// The URLs for all different YCharts data points we need to fetch
struct YChartsEndpoints {
    monthly_return: &'static str,
    quarterly_dividend: &'static str,
    current_eps: &'static str,
    forward_eps: &'static str,
    cape: &'static str,
}

// Initialize with all the endpoints we need to test
impl Default for YChartsEndpoints {
    fn default() -> Self {
        YChartsEndpoints {
            monthly_return: "https://ycharts.com/indicators/sp_500_monthly_total_return",
            quarterly_dividend: "https://ycharts.com/indicators/sp_500_dividends_per_share",
            current_eps: "https://ycharts.com/indicators/sp_500_eps",
            forward_eps: "https://ycharts.com/indicators/sp_500_earnings_per_share_forward_estimate",
            cape: "https://ycharts.com/indicators/cyclically_adjusted_pe_ratio",
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    env_logger::init();
    
    info!("Starting comprehensive YCharts fetch test");
    
    let endpoints = YChartsEndpoints::default();
    let urls = [
        ("Monthly Return", endpoints.monthly_return),
        ("Quarterly Dividend", endpoints.quarterly_dividend),
        ("Current EPS", endpoints.current_eps),
        ("Forward EPS", endpoints.forward_eps),
        ("CAPE", endpoints.cape),
    ];
    
    // Test the original function for comparison
    info!("TESTING ORIGINAL FUNCTION:");
    for (name, url) in urls.iter() {
        info!("-----------------------------------------------------");
        info!("Testing {}", name);
        match fetch_ycharts_value_original(url).await {
            Ok((period, value)) => {
                info!("SUCCESS: Original function found {} of {} for period {}", name, value, period);
            },
            Err(e) => {
                error!("ERROR: Original function failed to fetch {}: {}", name, e);
            }
        }
    }
    
    // Test the improved function
    info!("\n\nTESTING IMPROVED FUNCTION:");
    for (name, url) in urls.iter() {
        info!("-----------------------------------------------------");
        info!("Testing {}", name);
        match fetch_ycharts_value_improved(url).await {
            Ok((period, value)) => {
                info!("SUCCESS: Improved function found {} of {} for period {}", name, value, period);
            },
            Err(e) => {
                error!("ERROR: Improved function failed to fetch {}: {}", name, e);
            }
        }
    }
    
    Ok(())
}

// The original implementation (as close as possible to your current code)
async fn fetch_ycharts_value_original(url: &str) -> Result<(String, f64), Box<dyn Error>> {
    info!("Original function fetching data from URL: {}", url);
    
    let client = Client::new();
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
    
    info!("Original function found stat text: {}", stat);

    // This is your current regex pattern
    let re = Regex::new(r"([-+]?\d*\.?\d+)\s*(?:USD)?\s*(?:for\s+)?(?:Q\d\s+\d{4}|(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{4})")?;
    
    let (value, period_text) = match re.captures(stat) {
        Some(caps) => {
            let value_str = caps.get(1).ok_or("No value match")?.as_str();
            let full_match = caps.get(0).ok_or("No full match")?.as_str();
            let period_part = full_match.split_whitespace().skip(1).collect::<Vec<_>>().join(" ");
            info!("Original regex: Parsed value: {}, period text: {}", value_str, period_part);
            (value_str.parse::<f64>()?, period_part)
        },
        None => {
            // Simple fallback to just extract a number
            let simple_re = Regex::new(r"([-+]?\d*\.?\d+)")?;
            if let Some(caps) = simple_re.captures(stat) {
                let value_str = caps.get(1).ok_or("No value match with simple regex")?.as_str();
                error!("Original regex failed, using fallback. Value: {}, Raw text: {}", value_str, stat);
                (value_str.parse::<f64>()?, "Unknown".to_string())
            } else {
                error!("Original: All regex attempts failed for: {}", stat);
                return Err("Failed to parse value and period".into());
            }
        }
    };

    // Handle special case for monthly returns
    let final_value = if url.contains("monthly_total_return") && stat.contains('%') {
        info!("Original: Converting percentage to decimal for monthly return");
        value / 100.0
    } else {
        value
    };
    
    Ok((period_text, final_value))
}

// The improved implementation
async fn fetch_ycharts_value_improved(url: &str) -> Result<(String, f64), Box<dyn Error>> {
    info!("Improved function fetching data from URL: {}", url);
    
    let client = Client::new();
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
    
    info!("Improved function found stat text: {}", stat);

    // IMPROVED REGEX - handles the current YCharts format better
    let re = Regex::new(r"([-+]?\d*\.?\d+)%?\s*(?:USD)?\s*(?:for)?\s+(?:(Q\d)\s+(\d{4})|(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{4}))")?;
    
    if let Some(caps) = re.captures(stat) {
        let value_str = caps.get(1).ok_or("No value match")?.as_str();
        let value = value_str.parse::<f64>()?;
        
        let period_text = if let Some(quarter) = caps.get(2) {
            // It's quarterly data: Q1 2024 format
            let year = caps.get(3).unwrap().as_str();
            let quarter_text = format!("{}{}", year, quarter.as_str());
            info!("Improved: Parsed quarterly period: {}", quarter_text);
            quarter_text
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
            let formatted = format!("{}-{}", year, month_num);
            info!("Improved: Parsed monthly period: {} -> {}", month, formatted);
            formatted
        };
        
        info!("Improved: Final parsed value: {}, period: {}", value, period_text);
        
        // Convert percentage to decimal if needed
        let final_value = if stat.contains('%') {
            info!("Improved: Converting percentage to decimal");
            value / 100.0
        } else {
            value
        };
        
        return Ok((period_text, final_value));
    }
    
    // If the main regex didn't match, try a simpler one
    info!("Main regex didn't match, trying fallback patterns");
    
    // Fallback patterns for different formats
    let fallback_patterns = [
        // For values with just a number and % but no clear period
        r"([-+]?\d*\.?\d+)%",
        // For values with just a number
        r"([-+]?\d*\.?\d+)",
        // For other potential formats
        r"([-+]?\d*\.?\d+)\s+USD",
    ];
    
    for pattern in fallback_patterns {
        let fallback_re = Regex::new(pattern)?;
        if let Some(caps) = fallback_re.captures(stat) {
            let value_str = caps.get(1).ok_or("No value match in fallback")?.as_str();
            let value = value_str.parse::<f64>()?;
            let final_value = if stat.contains('%') {
                value / 100.0
            } else {
                value
            };
            
            info!("Improved: Fallback regex matched. Pattern: {}, Value: {}", pattern, value_str);
            
            // Try to extract a period from the remaining text
            let period_text = extract_period_from_remaining_text(stat);
            return Ok((period_text, final_value));
        }
    }
    
    // If we get here, all regex attempts failed
    error!("Improved: All regex patterns failed for: {}", stat);
    Err("Failed to parse value and period".into())
}

// Helper function to try to extract a period from text even if main regex fails
fn extract_period_from_remaining_text(text: &str) -> String {
    // Try to find a year pattern
    let year_re = Regex::new(r"\b(20\d{2})\b").unwrap();
    if let Some(caps) = year_re.captures(text) {
        let year = caps.get(1).unwrap().as_str();
        
        // Try to find a month or quarter near the year
        let month_re = Regex::new(r"\b(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\b").unwrap();
        if let Some(month_caps) = month_re.captures(text) {
            let month = month_caps.get(1).unwrap().as_str();
            return format!("{} {}", month, year);
        }
        
        let quarter_re = Regex::new(r"\b(Q[1-4])\b").unwrap();
        if let Some(q_caps) = quarter_re.captures(text) {
            let quarter = q_caps.get(1).unwrap().as_str();
            return format!("{} {}", quarter, year);
        }
        
        return format!("Unknown period {}", year);
    }
    
    "Unknown period".to_string()
}
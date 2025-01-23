//src/services/equity.rs
use reqwest::{self, Client};
use scraper::{Html, Selector};
use serde::Serialize;
use std::error::Error;
use log::info;
use regex::Regex;
use chrono::NaiveDate;
use serde::Serializer;

#[derive(Debug, Serialize)]
pub struct YChartsData {
    value: f64,
    #[serde(serialize_with = "serialize_date")]
    date: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct MarketData {
    sp500_price: f64,
    ttm_dividend: YChartsData,
    current_eps: YChartsData,
    forward_eps: YChartsData,
    cape: YChartsData,
}

fn serialize_date<S>(date: &NaiveDate, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&date.format("%Y-%m-%d").to_string())
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

pub async fn fetch_ycharts_value(url: &str, indicator: &str) -> Result<YChartsData, Box<dyn Error>> {
    info!("Fetching {} from {}", indicator, url);
    
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
        .ok_or_else(|| format!("Failed to find stat for {}", indicator))?
        .trim();

    let re_quarterly = Regex::new(r"(\d+\.?\d*)\s+USD\s+for\s+Q(\d)\s+(\d{4})")?;
    let re_monthly = Regex::new(r"(\d+\.?\d*)\s+for\s+([A-Za-z]+)\s+(\d{4})")?;

    let (value, date) = if let Some(caps) = re_quarterly.captures(stat) {
        let value = caps[1].parse::<f64>()?;
        let quarter = caps[2].parse::<u32>()?;
        let year = caps[3].parse::<i32>()?;
        
        let (month, day) = match quarter {
            1 => (3, 31),
            2 => (6, 30),
            3 => (9, 30),
            4 => (12, 31),
            _ => return Err("Invalid quarter".into())
        };
        
        let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        (value, date)
    } else if let Some(caps) = re_monthly.captures(stat) {
        let value = caps[1].parse::<f64>()?;
        let month = match &caps[2] {
            "Jan" => 1, "Feb" => 2, "Mar" => 3, "Apr" => 4,
            "May" => 5, "Jun" => 6, "Jul" => 7, "Aug" => 8,
            "Sep" => 9, "Oct" => 10, "Nov" => 11, "Dec" => 12,
            _ => return Err("Invalid month".into()),
        };
        let year = caps[3].parse::<i32>()?;
        let date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        (value, date)
    } else {
        return Err(format!("Failed to parse stat format: {}", stat).into());
    };

    Ok(YChartsData { value, date })
}

pub async fn get_market_data() -> Result<MarketData, Box<dyn Error>> {
    let sp500_price = fetch_sp500_price().await?;
    
    let ttm_dividend = fetch_ycharts_value(
        "https://ycharts.com/indicators/sp_500_dividends_per_share",
        "TTM dividend"
    ).await?;
    
    let current_eps = fetch_ycharts_value(
        "https://ycharts.com/indicators/sp_500_eps",
        "current EPS"
    ).await?;
    
    let forward_eps = fetch_ycharts_value(
        "https://ycharts.com/indicators/sp_500_earnings_per_share_forward_estimate",
        "forward EPS"
    ).await?;
    
    let cape = fetch_ycharts_value(
        "https://ycharts.com/indicators/cyclically_adjusted_pe_ratio",
        "CAPE"
    ).await?;

    Ok(MarketData {
        sp500_price,
        ttm_dividend,
        current_eps,
        forward_eps,
        cape,
    })
}
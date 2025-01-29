// src/services/sheets.rs
use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;
use crate::models::{MarketCache, QuarterlyData, HistoricalRecord};

// Configuration struct for Google Sheets
#[derive(Clone)]
pub struct SheetsConfig {
    pub spreadsheet_id: String,
    pub api_key: String,
}

// Represents the structure of our sheets
pub struct SheetNames {
    pub market_cache: &'static str,
    pub quarterly_data: &'static str,
    pub historical_data: &'static str,
}

impl Default for SheetNames {
    fn default() -> Self {
        SheetNames {
            market_cache: "MarketCache",
            quarterly_data: "QuarterlyData",
            historical_data: "HistoricalData",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawMarketCache {
    pub timestamp_yahoo: String,
    pub timestamp_ycharts: String,
    pub daily_close_sp500_price: f64,
    pub current_sp500_price: f64,
    pub current_cape: f64,
    pub cape_period: String,
}

pub struct SheetsStore {
    config: SheetsConfig,
    client: reqwest::Client,
    sheet_names: SheetNames,
}

impl SheetsStore {
    pub fn new(config: SheetsConfig) -> Self {
        SheetsStore {
            config,
            client: reqwest::Client::new(),
            sheet_names: SheetNames::default(),
        }
    }

    pub async fn get_market_cache(&self) -> Result<RawMarketCache, Box<dyn Error>> {
        let range = format!("{}!A2:F2", self.sheet_names.market_cache);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?key={}",
            self.config.spreadsheet_id, range, self.config.api_key
        );

        let response: serde_json::Value = self.client.get(&url).send().await?.json().await?;
        
        if let Some(values) = response["values"].as_array() {
            if let Some(row) = values.first() {
                return Ok(RawMarketCache {
                    timestamp_yahoo: row[0].as_str().unwrap_or("").to_string(),
                    timestamp_ycharts: row[1].as_str().unwrap_or("").to_string(),
                    daily_close_sp500_price: row[2].as_str().unwrap_or("0").parse()?,
                    current_sp500_price: row[3].as_str().unwrap_or("0").parse()?,
                    current_cape: row[4].as_str().unwrap_or("0").parse()?,
                    cape_period: row[5].as_str().unwrap_or("").to_string(),
                });
            }
        }

        Err("No market cache data found".into())
    }

    pub async fn update_market_cache(&self, cache: &RawMarketCache) -> Result<(), Box<dyn Error>> {
        let range = format!("{}!A2:F2", self.sheet_names.market_cache);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?valueInputOption=RAW&key={}",
            self.config.spreadsheet_id, range, self.config.api_key
        );

        let values = vec![vec![
            cache.timestamp_yahoo.to_string(),
            cache.timestamp_ycharts.to_string(),
            cache.daily_close_sp500_price.to_string(),
            cache.current_sp500_price.to_string(),
            cache.current_cape.to_string(),
            cache.cape_period.clone(),
        ]];

        let body = serde_json::json!({
            "values": values,
        });

        self.client.put(&url).json(&body).send().await?;
        Ok(())
    }

    pub async fn get_quarterly_data(&self) -> Result<Vec<QuarterlyData>, Box<dyn Error>> {
        let range = format!("{}!A2:D", self.sheet_names.quarterly_data);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?key={}",
            self.config.spreadsheet_id, range, self.config.api_key
        );

        let response: serde_json::Value = self.client.get(&url).send().await?.json().await?;
        
        let mut quarterly_data = Vec::new();
        if let Some(values) = response["values"].as_array() {
            for row in values {
                quarterly_data.push(QuarterlyData {
                    quarter: row[0].as_str().unwrap_or("").to_string(),
                    dividend: row[1].as_str().and_then(|s| s.parse().ok()),
                    eps_actual: row[2].as_str().and_then(|s| s.parse().ok()),
                    eps_estimated: row[3].as_str().and_then(|s| s.parse().ok()),
                });
            }
        }

        Ok(quarterly_data)
    }

    pub async fn update_quarterly_data(&self, data: &[QuarterlyData]) -> Result<(), Box<dyn Error>> {
        let range = format!("{}!A2:D{}", self.sheet_names.quarterly_data, data.len() + 1);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?valueInputOption=RAW&key={}",
            self.config.spreadsheet_id, range, self.config.api_key
        );

        let values: Vec<Vec<String>> = data.iter().map(|row| {
            vec![
                row.quarter.clone(),
                row.dividend.map(|v| v.to_string()).unwrap_or_default(),
                row.eps_actual.map(|v| v.to_string()).unwrap_or_default(),
                row.eps_estimated.map(|v| v.to_string()).unwrap_or_default(),
            ]
        }).collect();

        let body = serde_json::json!({
            "values": values,
        });

        self.client.put(&url).json(&body).send().await?;
        Ok(())
    }

    pub async fn get_historical_data(&self) -> Result<Vec<HistoricalRecord>, Box<dyn Error>> {
        let range = format!("{}!A2:E", self.sheet_names.historical_data);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?key={}",
            self.config.spreadsheet_id, range, self.config.api_key
        );

        let response: serde_json::Value = self.client.get(&url).send().await?.json().await?;
        
        let mut historical_data = Vec::new();
        if let Some(values) = response["values"].as_array() {
            for row in values {
                historical_data.push(HistoricalRecord {
                    year: row[0].as_str().unwrap_or("0").parse()?,
                    sp500_price: row[1].as_str().unwrap_or("0").parse()?,
                    dividend: row[2].as_str().unwrap_or("0").parse()?,
                    eps: row[3].as_str().unwrap_or("0").parse()?,
                    cape: row[4].as_str().unwrap_or("0").parse()?,
                });
            }
        }

        Ok(historical_data)
    }

    pub async fn update_historical_record(&self, record: &HistoricalRecord) -> Result<(), Box<dyn Error>> {
        // First get all records to find the correct row to update
        let all_records = self.get_historical_data().await?;
        let row_index = all_records.iter().position(|r| r.year == record.year)
            .ok_or("Record not found")?;

        let range = format!("{}!A{}:E{}", 
            self.sheet_names.historical_data,
            row_index + 2,  // +2 because sheet indices start at 1 and we have a header row
            row_index + 2
        );

        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?valueInputOption=RAW&key={}",
            self.config.spreadsheet_id, range, self.config.api_key
        );

        let values = vec![vec![
            record.year.to_string(),
            record.sp500_price.to_string(),
            record.dividend.to_string(),
            record.eps.to_string(),
            record.cape.to_string(),
        ]];

        let body = serde_json::json!({
            "values": values,
        });

        self.client.put(&url).json(&body).send().await?;
        Ok(())
    }
}
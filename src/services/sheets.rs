// src/services/sheets.rs

use serde::{Deserialize, Serialize};
use std::error::Error;
use crate::{models::QuarterlyData, services::google_oauth::fetch_access_token_from_file};
use log::info;
use serde_json::json;
use reqwest::Client;
use crate::models::HistoricalRecord;

#[derive(Clone)]
pub struct SheetsConfig {
    pub spreadsheet_id: String,
    // Instead of `api_key`, store the path to your service account JSON
    pub service_account_json_path: String,
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
    pub config: SheetsConfig,
    client: Client,
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

    pub async fn get_auth_token(&self) -> Result<String, Box<dyn Error>> {
        crate::services::google_oauth::fetch_access_token_from_file(&self.config.service_account_json_path).await
    }

    pub async fn bulk_upload_historical_records(&self, records: &[HistoricalRecord]) -> Result<(), Box<dyn Error>> {
        let token = self.get_auth_token().await?;
        let client = reqwest::Client::new();
        
        // Convert records to values maintaining exact CSV column order
        let values: Vec<Vec<String>> = records.iter()
            .map(|record| vec![
                record.year.to_string(),
                record.sp500_price.to_string(),
                record.dividend.to_string(),
                record.dividend_yield.to_string(),
                record.eps.to_string(),
                record.cape.to_string(),
                record.inflation.to_string(),
                record.total_return.to_string(),
                record.cumulative_return.to_string(),
            ])
            .collect();
    
        let range = format!("{}!A2:I{}", self.sheet_names.historical_data, values.len() + 1);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
            self.config.spreadsheet_id,
            range
        );
    
        let body = json!({
            "values": values,
            "majorDimension": "ROWS"
        });
    
        let response = client
            .put(&url)
            .header("Content-Type", "application/json")
            .query(&[("valueInputOption", "RAW")])
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?;
    
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Failed to upload historical records: {}", error_text).into());
        }
    
        Ok(())
    }    

    /// Example of reading from the "MarketCache!A2:F2" range
    pub async fn get_market_cache(&self) -> Result<RawMarketCache, Box<dyn Error>> {
        // 1. Fetch a token from the JSON
        let token = fetch_access_token_from_file(&self.config.service_account_json_path).await?;

        let range = format!("{}!A2:F2", self.sheet_names.market_cache);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
            self.config.spreadsheet_id, range
        );

        // 2. Bearer auth
        let response: serde_json::Value = self.client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        if let Some(values) = response["values"].as_array() {
            if let Some(row) = values.first() {
                return Ok(RawMarketCache {
                    timestamp_yahoo: row.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    timestamp_ycharts: row.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    daily_close_sp500_price: row.get(2).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    current_sp500_price: row.get(3).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    current_cape: row.get(4).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    cape_period: row.get(5).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                });
            }
        }

        Err("No market cache data found".into())
    }

    /// Example of updating the "MarketCache!A2:F2" range
    pub async fn update_market_cache(&self, cache: &RawMarketCache) -> Result<(), Box<dyn Error>> {
        let token = fetch_access_token_from_file(&self.config.service_account_json_path).await?;

        let range = format!("{}!A2:F2", self.sheet_names.market_cache);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?valueInputOption=RAW",
            self.config.spreadsheet_id, range
        );

        let values = vec![vec![
            cache.timestamp_yahoo.to_string(),
            cache.timestamp_ycharts.to_string(),
            cache.daily_close_sp500_price.to_string(),
            cache.current_sp500_price.to_string(),
            cache.current_cape.to_string(),
            cache.cape_period.clone(),
        ]];

        let body = json!({
            "values": values,
        });

        let resp = self.client
            .put(&url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        info!("update_market_cache response: {:?}", resp);
        Ok(())
    }

    /// Example of reading from "QuarterlyData!A2:D" range
    pub async fn get_quarterly_data(&self) -> Result<Vec<QuarterlyData>, Box<dyn Error>> {
        let token = fetch_access_token_from_file(&self.config.service_account_json_path).await?;

        let range = format!("{}!A2:D", self.sheet_names.quarterly_data);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
            self.config.spreadsheet_id, range
        );

        let response: serde_json::Value = self.client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let mut quarterly_data = Vec::new();
        if let Some(values) = response["values"].as_array() {
            for row in values {
                let quarter = row.get(0).and_then(|v| v.as_str()).unwrap_or("");
                let dividend = row.get(1).and_then(|v| v.as_str()).unwrap_or("").parse().ok();
                let eps_actual = row.get(2).and_then(|v| v.as_str()).unwrap_or("").parse().ok();
                let eps_estimated = row.get(3).and_then(|v| v.as_str()).unwrap_or("").parse().ok();

                quarterly_data.push(QuarterlyData {
                    quarter: quarter.to_string(),
                    dividend,
                    eps_actual,
                    eps_estimated,
                });
            }
        }
        Ok(quarterly_data)
    }

    pub async fn update_quarterly_data(&self, data: &[QuarterlyData]) -> Result<(), Box<dyn Error>> {
        let token = fetch_access_token_from_file(&self.config.service_account_json_path).await?;

        let range = format!("{}!A2:D{}", self.sheet_names.quarterly_data, data.len() + 1);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?valueInputOption=RAW",
            self.config.spreadsheet_id, range
        );

        let values: Vec<Vec<String>> = data.iter().map(|row| {
            vec![
                row.quarter.clone(),
                row.dividend.map(|v| v.to_string()).unwrap_or_default(),
                row.eps_actual.map(|v| v.to_string()).unwrap_or_default(),
                row.eps_estimated.map(|v| v.to_string()).unwrap_or_default(),
            ]
        }).collect();

        let body = json!({
            "values": values,
        });

        let resp = self.client
            .put(&url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        info!("update_quarterly_data response: {:?}", resp);
        Ok(())
    }

    pub async fn get_historical_data(&self) -> Result<Vec<HistoricalRecord>, Box<dyn Error>> {
        let token = fetch_access_token_from_file(&self.config.service_account_json_path).await?;
    
        // Range includes all columns A through I
        let range = format!("{}!A2:I", self.sheet_names.historical_data);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
            self.config.spreadsheet_id, range
        );
    
        let response: serde_json::Value = self.client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
    
        let mut historical_data = Vec::new();
        if let Some(values) = response["values"].as_array() {
            for row in values {
                historical_data.push(HistoricalRecord {
                    year: row.get(0).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    sp500_price: row.get(1).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    dividend: row.get(2).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    dividend_yield: row.get(3).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    eps: row.get(4).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    cape: row.get(5).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    inflation: row.get(6).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    total_return: row.get(7).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    cumulative_return: row.get(8).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                });
            }
        }
    
        Ok(historical_data)
    }

    pub async fn update_historical_record(&self, record: &HistoricalRecord) -> Result<(), Box<dyn Error>> {
        // fetch all records to find the matching row
        let all_records = self.get_historical_data().await?;
        let row_index = all_records.iter().position(|r| r.year == record.year)
            .ok_or("Record not found")?;
    
        let token = fetch_access_token_from_file(&self.config.service_account_json_path).await?;
    
        // +2 because first row is headers
        let row_num = row_index + 2;
        let range = format!("{}!A{}:I{}", self.sheet_names.historical_data, row_num, row_num);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?valueInputOption=RAW",
            self.config.spreadsheet_id, range
        );
    
        let values = vec![vec![
            record.year.to_string(),
            record.sp500_price.to_string(),
            record.dividend.to_string(),
            record.dividend_yield.to_string(),
            record.eps.to_string(),
            record.cape.to_string(),
            record.inflation.to_string(),
            record.total_return.to_string(),
            record.cumulative_return.to_string(),
        ]];
    
        let body = json!({
            "values": values,
        });
    
        let response = self.client
            .put(&url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
    
        info!("update_historical_record response: {:?}", response);
        Ok(())
    }
}

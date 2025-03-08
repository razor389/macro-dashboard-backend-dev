// src/services/calculations.rs
use serde::Serialize;
use log::warn;
use crate::models::HistoricalRecord;
use anyhow::Result;

#[derive(Serialize)]
pub struct MarketMetrics {
    pub avg_dividend_yield: f64,
    pub past_inflation_cagr: f64,
    pub current_inflation_cagr: f64,
    pub past_earnings_cagr: f64,
    pub current_earnings_cagr: f64,
    pub past_cape_cagr: f64,
    pub current_cape_cagr: f64,
    pub past_returns_cagr: f64,
    pub current_returns_cagr: f64,
}

fn calculate_cagr(start_value: f64, end_value: f64, years: f64) -> f64 {
    if start_value <= 0.0 || end_value <= 0.0 || years <= 0.0 {
        0.0
    } else {
        (end_value / start_value).powf(1.0 / years) - 1.0
    }
}

fn calculate_average(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

pub fn calculate_market_metrics(historical_data: &[HistoricalRecord]) -> Result<MarketMetrics> {
    let mut sorted_data = historical_data.to_vec();
    sorted_data.sort_by_key(|r| r.year);

    // Calculate average dividend yield
    let dividend_yields: Vec<f64> = sorted_data.iter()
        .filter(|r| r.dividend_yield > 0.0)
        .map(|r| r.dividend_yield)
        .collect();
    let avg_dividend_yield = calculate_average(&dividend_yields);

    // Helper to compute CAGRs for a metric with validation and logging
    fn compute_cagrs(
        data: &[HistoricalRecord],
        metric_extractor: fn(&HistoricalRecord) -> f64,
        metric_name: &'static str,
    ) -> (f64, f64) {
        let valid_entries: Vec<&HistoricalRecord> = data.iter()
            .filter(|r| metric_extractor(r) > 0.0)
            .collect();
    
        let (past_cagr, current_cagr) = if valid_entries.len() < 2 {
            warn!("Insufficient valid {} data points ({}) for CAGR calculation", metric_name, valid_entries.len());
            (0.0, 0.0)
        } else {
            // Calculate past CAGR (full period)
            let first = valid_entries.first().unwrap();
            let last = valid_entries.last().unwrap();
            let past_years = (last.year - first.year) as f64;
            let past_cagr = calculate_cagr(metric_extractor(first), metric_extractor(last), past_years);
    
            // Calculate current CAGR (10-year window)
            let target_start_year = last.year - 10; // Use the last valid entry's year -10
            let start = valid_entries.iter()
                .take_while(|r| r.year <= target_start_year)
                .last();
    
            let current_cagr = match start {
                Some(start_entry) => {
                    let years = (last.year - start_entry.year) as f64;
                    calculate_cagr(metric_extractor(start_entry), metric_extractor(last), years)
                }
                None => {
                    warn!("No valid {} start point found for 10-year CAGR calculation", metric_name);
                    0.0
                }
            };
    
            (past_cagr, current_cagr)
        };
    
        (past_cagr, current_cagr)
    }

    // Calculate metrics for each category
    let (past_inflation_cagr, current_inflation_cagr) = 
        compute_cagrs(&sorted_data, |r| r.inflation, "inflation");
    let (past_earnings_cagr, current_earnings_cagr) = 
        compute_cagrs(&sorted_data, |r| r.eps, "earnings");
    let (past_cape_cagr, current_cape_cagr) = 
        compute_cagrs(&sorted_data, |r| r.cape, "CAPE");
    let (past_returns_cagr, current_returns_cagr) = 
        compute_cagrs(&sorted_data, |r| r.cumulative_return, "returns");

    Ok(MarketMetrics {
        avg_dividend_yield,
        past_inflation_cagr,
        current_inflation_cagr,
        past_earnings_cagr,
        current_earnings_cagr,
        past_cape_cagr,
        current_cape_cagr,
        past_returns_cagr,
        current_returns_cagr,
    })
}
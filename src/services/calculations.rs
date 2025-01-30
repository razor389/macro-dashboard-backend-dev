// src/services/calculations.rs

use serde::Serialize;

use crate::models::HistoricalRecord;
use std::error::Error;

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
        return 0.0;
    }
    (end_value / start_value).powf(1.0 / years) - 1.0
}

fn calculate_average(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

pub fn calculate_market_metrics(historical_data: &[HistoricalRecord]) -> Result<MarketMetrics, Box<dyn Error>> {
    // Sort data by year to ensure proper ordering
    let mut sorted_data = historical_data.to_vec();
    sorted_data.sort_by_key(|r| r.year);

    // Calculate average dividend yield
    let dividend_yields: Vec<f64> = sorted_data.iter()
        .filter(|r| r.dividend_yield > 0.0)
        .map(|r| r.dividend_yield)
        .collect();
    let avg_dividend_yield = calculate_average(&dividend_yields);

    // Get the most recent year's data
    let current_year = sorted_data.last()
        .ok_or("No historical data available")?
        .year;

    // Find data for 10 years ago
    let ten_years_ago = current_year - 10;
    let ten_year_data = sorted_data.iter()
        .find(|r| r.year == ten_years_ago)
        .ok_or("10-year historical data not available")?;

    // Calculate past and current inflation CAGR
    let first_inflation = sorted_data.first()
        .ok_or("No historical data available")?
        .inflation;
    let current_inflation = sorted_data.last()
        .ok_or("No current data available")?
        .inflation;
    let ten_year_ago_inflation = ten_year_data.inflation;

    let inflation_years = (current_year - sorted_data[0].year) as f64;
    let past_inflation_cagr = calculate_cagr(first_inflation, current_inflation, inflation_years);
    let current_inflation_cagr = calculate_cagr(ten_year_ago_inflation, current_inflation, 10.0);

    // Calculate earnings growth CAGR
    let first_earnings = sorted_data.first().unwrap().eps;
    let current_earnings = sorted_data.last().unwrap().eps;
    let ten_year_ago_earnings = ten_year_data.eps;

    let earnings_years = (current_year - sorted_data[0].year) as f64;
    let past_earnings_cagr = calculate_cagr(first_earnings, current_earnings, earnings_years);
    let current_earnings_cagr = calculate_cagr(ten_year_ago_earnings, current_earnings, 10.0);

    // Calculate CAPE CAGR
    let first_cape = sorted_data.first().unwrap().cape;
    let current_cape = sorted_data.last().unwrap().cape;
    let ten_year_ago_cape = ten_year_data.cape;

    let cape_years = (current_year - sorted_data[0].year) as f64;
    let past_cape_cagr = calculate_cagr(first_cape, current_cape, cape_years);
    let current_cape_cagr = calculate_cagr(ten_year_ago_cape, current_cape, 10.0);

    // Calculate returns CAGR
    let first_return = sorted_data.first().unwrap().cumulative_return;
    let current_return = sorted_data.last().unwrap().cumulative_return;
    let ten_year_ago_return = ten_year_data.cumulative_return;

    let return_years = (current_year - sorted_data[0].year) as f64;
    let past_returns_cagr = calculate_cagr(first_return, current_return, return_years);
    let current_returns_cagr = calculate_cagr(ten_year_ago_return, current_return, 10.0);

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
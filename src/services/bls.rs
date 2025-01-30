use reqwest;
use serde::Deserialize;
use std::env;
use dotenv::dotenv;
use std::error::Error as StdError;
pub type Result<T> = std::result::Result<T, Box<dyn StdError + Send + Sync>>;
use std::fmt;
use log::{info, error};  // Import the logging macros

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct BlsResponse {
    status: String,
    Results: Results,
}

#[derive(Deserialize, Debug)]
struct Results {
    series: Vec<Series>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct Series {
    seriesID: String,
    data: Vec<DataPoint>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct DataPoint {
    year: String,
    period: String,
    value: String,
}

// Define a custom error type
#[derive(Debug)]
struct DataFetchError {
    details: String,
}

impl DataFetchError {
    fn new(msg: &str) -> DataFetchError {
        DataFetchError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for DataFetchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl StdError for DataFetchError {}

pub async fn fetch_inflation_data() -> Result<f64> {
    dotenv().ok();  // Load environment variables from .env file
    
    let api_key = env::var("BLS_API_KEY").expect("BLS_API_KEY must be set");
    info!("API Key loaded successfully");

    let url = "https://api.bls.gov/publicAPI/v1/timeseries/data/";
    info!("Request URL: {}", url);

    let request_body = serde_json::json!({
        "seriesid": ["CUUR0000SA0"],
        "registrationkey": api_key
    });
    info!("Request Body: {:?}", request_body);

    let client = reqwest::Client::new();
    let response = client.post(url)
        .json(&request_body)
        .send()
        .await?;
        
    let response_text = response.text().await?;
    info!("Response Text: {}", response_text);

    let resp: BlsResponse = serde_json::from_str(&response_text)?;
    info!("Parsed Response: {:?}", resp);

    if let Some(series) = resp.Results.series.first() {
        info!("Series Data: {:?}", series.data);

        // Get the most recent data point
        if let Some(current_data) = series.data.first() {
            let current_year = &current_data.year;
            let current_period = &current_data.period;
            let current_value: f64 = current_data.value.parse().unwrap_or(0.0);

            info!("Current Year: {}, Current Period: {}, Current Value: {}", current_year, current_period, current_value);

            // Find the data point from the same month last year
            if let Some(last_year_data) = series.data.iter().find(|d| {
                &d.year == &(current_year.parse::<i32>().unwrap() - 1).to_string() && &d.period == current_period
            }) {
                let last_year_value: f64 = last_year_data.value.parse().unwrap_or(0.0);

                info!("Last Year Value: {}", last_year_value);

                // Calculate the yearly percentage change
                let percentage_change = ((current_value - last_year_value) / last_year_value) * 100.0;
                info!("Yearly Percentage Change: {}", percentage_change);
                return Ok(percentage_change);
            } else {
                error!("No data found for the same month last year.");
            }
        } else {
            error!("No current data found.");
        }
    } else {
        error!("No series data found.");
    }

    Err(Box::new(DataFetchError::new("No data found")))
}

use reqwest::Client;
use regex::Regex;
use log::{info, error};
use env_logger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    info!("Debugging Yahoo Finance HTML structure...");
    
    let url = "https://finance.yahoo.com/quote/%5EGSPC";
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()?;
        
    let resp = client.get(url).send().await?.text().await?;
    
    // Look for the current regex pattern
    let re = Regex::new(r#"data-symbol="\^GSPC"[^>]*data-value="([0-9.]+)""#)?;
    if let Some(caps) = re.captures(&resp) {
        info!("Found price using current regex: {}", caps.get(1).unwrap().as_str());
    } else {
        error!("Current regex pattern not found");
        
        // Try some alternative patterns
        let patterns = vec![
            r#""regularMarketPrice":\{"raw":([0-9.]+),"fmt":"[^"]*"\}"#,
            r#""regularMarketPrice":\{"raw":([0-9.]+)"#,
            r#"data-reactid="[^"]*">([0-9,]+\.[0-9]+)</span>"#,
            r#"data-field="regularMarketPrice"[^>]*>([0-9,]+\.[0-9]+)"#,
            r#"Current price.*?([0-9,]+\.[0-9]+)"#,
            r#"<span[^>]*data-symbol="\^GSPC"[^>]*>([0-9,]+\.[0-9]+)</span>"#,
        ];
        
        for pattern in patterns {
            let re = Regex::new(pattern)?;
            if let Some(caps) = re.captures(&resp) {
                info!("Found price using pattern '{}': {}", pattern, caps.get(1).unwrap().as_str());
                break;
            }
        }
        
        // Look for any number that looks like a stock price
        let price_re = Regex::new(r"([0-9]{4}\.[0-9]{2})")?;
        let mut prices = Vec::new();
        for cap in price_re.captures_iter(&resp) {
            let price_str = cap.get(1).unwrap().as_str();
            if let Ok(price) = price_str.parse::<f64>() {
                if price > 3000.0 && price < 7000.0 {
                    prices.push(price);
                }
            }
        }
        
        if !prices.is_empty() {
            info!("Found potential prices: {:?}", prices);
        }
        
        // Save a sample of the HTML for inspection
        let sample = if resp.len() > 5000 {
            &resp[..5000]
        } else {
            &resp
        };
        
        info!("HTML sample (first 5000 chars):");
        println!("{}", sample);
    }
    
    Ok(())
}
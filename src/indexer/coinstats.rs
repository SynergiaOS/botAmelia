//! CoinStats API client for price data

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// CoinStats API client
pub struct CoinStatsClient {
    api_key: String,
    client: Client,
    base_url: String,
}

/// CoinStats price response
#[derive(Debug, Deserialize)]
struct PriceResponse {
    coins: Vec<CoinPrice>,
}

/// Individual coin price
#[derive(Debug, Deserialize)]
pub struct CoinPrice {
    id: String,
    symbol: String,
    name: String,
    price: f64,
    #[serde(rename = "priceChange1d")]
    price_change_1d: Option<f64>,
    #[serde(rename = "priceChange1w")]
    price_change_1w: Option<f64>,
    #[serde(rename = "marketCap")]
    market_cap: Option<f64>,
    volume: Option<f64>,
}

/// Portfolio response
#[derive(Debug, Deserialize)]
struct PortfolioResponse {
    #[serde(rename = "totalValue")]
    total_value: f64,
    #[serde(rename = "totalChange24h")]
    total_change_24h: f64,
    #[serde(rename = "totalChangePercentage24h")]
    total_change_percentage_24h: f64,
    coins: Vec<PortfolioCoin>,
}

/// Portfolio coin
#[derive(Debug, Deserialize)]
struct PortfolioCoin {
    id: String,
    symbol: String,
    amount: f64,
    value: f64,
    #[serde(rename = "change24h")]
    change_24h: f64,
    #[serde(rename = "changePercentage24h")]
    change_percentage_24h: f64,
}

/// Market data request
#[derive(Debug, Serialize)]
struct MarketDataRequest {
    coins: Vec<String>,
    currency: String,
}

impl CoinStatsClient {
    /// Creates new CoinStats client
    pub async fn new(api_key: String, client: Client) -> Result<Self> {
        let base_url = "https://openapiv1.coinstats.app".to_string();

        info!("CoinStats client initialized");
        Ok(Self {
            api_key,
            client,
            base_url,
        })
    }

    /// Gets prices for multiple symbols
    pub async fn get_prices(&self, symbols: &[String]) -> Result<HashMap<String, f64>> {
        if symbols.is_empty() {
            return Ok(HashMap::new());
        }

        let url = format!("{}/coins", self.base_url);
        let symbols_param = symbols.join(",");

        let response: PriceResponse = self
            .client
            .get(&url)
            .header("X-API-KEY", &self.api_key)
            .query(&[("symbols", symbols_param.as_str()), ("currency", "USD")])
            .send()
            .await
            .context("Failed to fetch prices from CoinStats")?
            .json()
            .await
            .context("Failed to parse CoinStats price response")?;

        let mut prices = HashMap::new();
        for coin in response.coins {
            prices.insert(coin.symbol.to_uppercase(), coin.price);
        }

        debug!("Fetched {} prices from CoinStats", prices.len());
        Ok(prices)
    }

    /// Gets price for a single symbol
    pub async fn get_price(&self, symbol: &str) -> Result<f64> {
        let prices = self.get_prices(&[symbol.to_string()]).await?;
        prices
            .get(&symbol.to_uppercase())
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Price not found for symbol: {}", symbol))
    }

    /// Gets market data for coins
    pub async fn get_market_data(&self, coin_ids: &[String]) -> Result<Vec<CoinPrice>> {
        let url = format!("{}/coins", self.base_url);
        let ids_param = coin_ids.join(",");

        let response: PriceResponse = self
            .client
            .get(&url)
            .header("X-API-KEY", &self.api_key)
            .query(&[("coinIds", ids_param.as_str()), ("currency", "USD")])
            .send()
            .await
            .context("Failed to fetch market data from CoinStats")?
            .json()
            .await
            .context("Failed to parse CoinStats market data response")?;

        Ok(response.coins)
    }

    /// Calculates portfolio value
    pub async fn calculate_portfolio_value(
        &self,
        holdings: &HashMap<String, f64>, // symbol -> amount
    ) -> Result<PortfolioValue> {
        if holdings.is_empty() {
            return Ok(PortfolioValue {
                total_value: 0.0,
                total_change_24h: 0.0,
                total_change_percentage_24h: 0.0,
                coin_values: HashMap::new(),
            });
        }

        let symbols: Vec<String> = holdings.keys().cloned().collect();
        let prices = self.get_prices(&symbols).await?;

        let mut total_value = 0.0;
        let mut coin_values = HashMap::new();

        for (symbol, amount) in holdings {
            if let Some(price) = prices.get(&symbol.to_uppercase()) {
                let value = amount * price;
                total_value += value;
                coin_values.insert(
                    symbol.clone(),
                    CoinValue {
                        amount: *amount,
                        price: *price,
                        value,
                        change_24h: 0.0, // Would need historical data
                        change_percentage_24h: 0.0,
                    },
                );
            }
        }

        Ok(PortfolioValue {
            total_value,
            total_change_24h: 0.0, // Would need historical data
            total_change_percentage_24h: 0.0,
            coin_values,
        })
    }

    /// Gets trending coins
    pub async fn get_trending_coins(&self, limit: u32) -> Result<Vec<CoinPrice>> {
        let url = format!("{}/coins", self.base_url);

        let response: PriceResponse = self
            .client
            .get(&url)
            .header("X-API-KEY", &self.api_key)
            .query(&[
                ("limit", limit.to_string().as_str()),
                ("currency", "USD"),
                ("sort", "rank"),
            ])
            .send()
            .await
            .context("Failed to fetch trending coins from CoinStats")?
            .json()
            .await
            .context("Failed to parse CoinStats trending response")?;

        Ok(response.coins)
    }

    /// Searches for coins by name or symbol
    pub async fn search_coins(&self, query: &str) -> Result<Vec<CoinPrice>> {
        let url = format!("{}/coins/search", self.base_url);

        let response: PriceResponse = self
            .client
            .get(&url)
            .header("X-API-KEY", &self.api_key)
            .query(&[("query", query), ("currency", "USD")])
            .send()
            .await
            .context("Failed to search coins on CoinStats")?
            .json()
            .await
            .context("Failed to parse CoinStats search response")?;

        Ok(response.coins)
    }

    /// Health check for CoinStats API
    pub async fn health_check(&self) -> Result<()> {
        // Try to fetch a simple price to test connectivity
        let _price = self
            .get_price("BTC")
            .await
            .context("CoinStats health check failed")?;

        Ok(())
    }

    /// Gets supported currencies
    pub async fn get_supported_currencies(&self) -> Result<Vec<String>> {
        // CoinStats typically supports these currencies
        Ok(vec![
            "USD".to_string(),
            "EUR".to_string(),
            "GBP".to_string(),
            "JPY".to_string(),
            "BTC".to_string(),
            "ETH".to_string(),
        ])
    }

    /// Gets rate limits info
    pub async fn get_rate_limits(&self) -> Result<RateLimitInfo> {
        // CoinStats rate limits (typical values)
        Ok(RateLimitInfo {
            requests_per_minute: 100,
            requests_per_hour: 1000,
            requests_per_day: 10000,
            current_usage: 0, // Would need to track
        })
    }
}

/// Portfolio value calculation result
#[derive(Debug, Serialize)]
pub struct PortfolioValue {
    pub total_value: f64,
    pub total_change_24h: f64,
    pub total_change_percentage_24h: f64,
    pub coin_values: HashMap<String, CoinValue>,
}

/// Individual coin value in portfolio
#[derive(Debug, Serialize)]
pub struct CoinValue {
    pub amount: f64,
    pub price: f64,
    pub value: f64,
    pub change_24h: f64,
    pub change_percentage_24h: f64,
}

/// Rate limit information
#[derive(Debug, Serialize)]
pub struct RateLimitInfo {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
    pub current_usage: u32,
}

/// Price alert configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct PriceAlert {
    pub symbol: String,
    pub target_price: f64,
    pub condition: AlertCondition,
    pub enabled: bool,
}

/// Alert condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    Above,
    Below,
    PercentageChange(f64),
}

impl CoinStatsClient {
    /// Checks price alerts
    pub async fn check_price_alerts(&self, alerts: &[PriceAlert]) -> Result<Vec<TriggeredAlert>> {
        let mut triggered = Vec::new();

        for alert in alerts.iter().filter(|a| a.enabled) {
            match self.get_price(&alert.symbol).await {
                Ok(current_price) => {
                    let should_trigger = match alert.condition {
                        AlertCondition::Above => current_price > alert.target_price,
                        AlertCondition::Below => current_price < alert.target_price,
                        AlertCondition::PercentageChange(_) => false, // Would need historical data
                    };

                    if should_trigger {
                        triggered.push(TriggeredAlert {
                            symbol: alert.symbol.clone(),
                            current_price,
                            target_price: alert.target_price,
                            condition: alert.condition.clone(),
                        });
                    }
                }
                Err(e) => {
                    warn!("Failed to check price for {}: {}", alert.symbol, e);
                }
            }
        }

        Ok(triggered)
    }
}

/// Triggered price alert
#[derive(Debug, Serialize)]
pub struct TriggeredAlert {
    pub symbol: String,
    pub current_price: f64,
    pub target_price: f64,
    pub condition: AlertCondition,
}

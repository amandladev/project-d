use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Source of an exchange rate — determines priority in 3-tier resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateSource {
    /// Compiled-in fallback rates (lowest priority).
    Bundled,
    /// Fetched from an API and cached locally.
    Cached,
    /// Manually set by the user (highest priority).
    UserOverride,
}

impl std::fmt::Display for RateSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateSource::Bundled => write!(f, "bundled"),
            RateSource::Cached => write!(f, "cached"),
            RateSource::UserOverride => write!(f, "user_override"),
        }
    }
}

impl RateSource {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "bundled" => Some(RateSource::Bundled),
            "cached" => Some(RateSource::Cached),
            "user_override" => Some(RateSource::UserOverride),
            _ => None,
        }
    }

    /// Priority for rate selection: higher = preferred.
    pub fn priority(&self) -> u8 {
        match self {
            RateSource::Bundled => 0,
            RateSource::Cached => 1,
            RateSource::UserOverride => 2,
        }
    }
}

/// An exchange rate between two currencies.
///
/// `rate` is stored as a fixed-point integer with 6 decimal places.
/// e.g., 1.234567 is stored as 1_234_567.
/// This avoids floating-point issues while keeping high precision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRate {
    pub from_currency: String,
    pub to_currency: String,
    /// Rate as fixed-point with 6 decimals: 1_000_000 = 1.0
    pub rate: i64,
    pub source: RateSource,
    pub fetched_at: DateTime<Utc>,
}

/// Precision multiplier for exchange rates (6 decimal places).
pub const RATE_PRECISION: i64 = 1_000_000;

impl ExchangeRate {
    pub fn new(
        from_currency: String,
        to_currency: String,
        rate: i64,
        source: RateSource,
    ) -> Self {
        Self {
            from_currency: from_currency.to_uppercase(),
            to_currency: to_currency.to_uppercase(),
            rate,
            source,
            fetched_at: Utc::now(),
        }
    }

    /// Convert an amount (in cents) from one currency to another.
    /// Returns the converted amount in cents.
    pub fn convert(&self, amount_cents: i64) -> i64 {
        // amount_cents * rate / RATE_PRECISION
        // Use i128 intermediate to avoid overflow
        let result = (amount_cents as i128 * self.rate as i128) / RATE_PRECISION as i128;
        result as i64
    }

    /// Get the rate as a float (for display purposes only).
    pub fn rate_as_f64(&self) -> f64 {
        self.rate as f64 / RATE_PRECISION as f64
    }
}

/// Result of a currency conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    pub original_amount: i64,
    pub converted_amount: i64,
    pub from_currency: String,
    pub to_currency: String,
    pub rate: i64,
    pub rate_source: RateSource,
    pub rate_fetched_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_simple() {
        // 1 USD = 0.85 EUR → rate = 850_000
        let rate = ExchangeRate::new("USD".into(), "EUR".into(), 850_000, RateSource::Bundled);
        // $100.00 = 10000 cents → 8500 cents = €85.00
        assert_eq!(rate.convert(10_000), 8_500);
    }

    #[test]
    fn test_convert_large_amount() {
        // 1 USD = 20.50 MXN → rate = 20_500_000
        let rate = ExchangeRate::new("USD".into(), "MXN".into(), 20_500_000, RateSource::Cached);
        // $1,000.00 = 100_000 cents → 2,050,000 cents = MXN 20,500.00
        assert_eq!(rate.convert(100_000), 2_050_000);
    }

    #[test]
    fn test_rate_source_priority() {
        assert!(RateSource::UserOverride.priority() > RateSource::Cached.priority());
        assert!(RateSource::Cached.priority() > RateSource::Bundled.priority());
    }

    #[test]
    fn test_rate_as_f64() {
        let rate = ExchangeRate::new("USD".into(), "EUR".into(), 850_000, RateSource::Bundled);
        assert!((rate.rate_as_f64() - 0.85).abs() < 1e-10);
    }

    #[test]
    fn test_currency_uppercased() {
        let rate = ExchangeRate::new("usd".into(), "eur".into(), 850_000, RateSource::Bundled);
        assert_eq!(rate.from_currency, "USD");
        assert_eq!(rate.to_currency, "EUR");
    }
}

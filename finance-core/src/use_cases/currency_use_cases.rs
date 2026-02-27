use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::entities::exchange_rate::{ConversionResult, ExchangeRate, RateSource, RATE_PRECISION};
use crate::errors::DomainError;
use crate::repositories::ExchangeRateRepository;

/// Bundled default exchange rates (base: USD).
/// These are compiled into the binary as a fallback when the user has never been online.
/// Rates are approximate as of early 2026.
const BUNDLED_RATES: &[(&str, &str, i64)] = &[
    ("USD", "EUR", 920_000),    // 0.92
    ("USD", "GBP", 790_000),    // 0.79
    ("USD", "JPY", 149_500_000), // 149.50
    ("USD", "MXN", 17_200_000), // 17.20
    ("USD", "CAD", 1_360_000),  // 1.36
    ("USD", "AUD", 1_540_000),  // 1.54
    ("USD", "CHF", 880_000),    // 0.88
    ("USD", "CNY", 7_250_000),  // 7.25
    ("USD", "BRL", 5_100_000),  // 5.10
    ("USD", "INR", 83_500_000), // 83.50
    ("USD", "KRW", 1_330_000_000), // 1330.0
    ("USD", "COP", 4_100_000_000), // 4100.0
    ("USD", "ARS", 900_000_000),   // 900.0
    ("USD", "PEN", 3_750_000),  // 3.75
    ("USD", "CLP", 950_000_000),   // 950.0
    // Inverse rates: EUR base
    ("EUR", "USD", 1_087_000),  // 1.087
    // GBP base
    ("GBP", "USD", 1_266_000),  // 1.266
    // MXN base
    ("MXN", "USD", 58_139),     // 0.058139
];

/// Rate freshness info returned to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateFreshness {
    pub source: RateSource,
    pub fetched_at: DateTime<Utc>,
    pub age_seconds: i64,
}

/// Currency conversion use cases.
pub struct CurrencyUseCases<'a> {
    rate_repo: &'a dyn ExchangeRateRepository,
}

impl<'a> CurrencyUseCases<'a> {
    pub fn new(rate_repo: &'a dyn ExchangeRateRepository) -> Self {
        Self { rate_repo }
    }

    /// Seed the database with bundled default rates (idempotent).
    pub fn seed_bundled_rates(&self) -> Result<usize, DomainError> {
        let rates: Vec<ExchangeRate> = BUNDLED_RATES
            .iter()
            .map(|(from, to, rate)| ExchangeRate {
                from_currency: from.to_string(),
                to_currency: to.to_string(),
                rate: *rate,
                source: RateSource::Bundled,
                fetched_at: Utc::now(),
            })
            .collect();

        let count = rates.len();
        self.rate_repo.save_batch(&rates)?;
        Ok(count)
    }

    /// Update cached rates from an external source (called by iOS after API fetch).
    /// `rates_json` is an array of `{ "from": "USD", "to": "EUR", "rate": 0.92 }`.
    pub fn update_cached_rates(&self, rates_json: &str) -> Result<usize, DomainError> {
        #[derive(Deserialize)]
        struct RateInput {
            from: String,
            to: String,
            rate: f64,
        }

        let inputs: Vec<RateInput> = serde_json::from_str(rates_json)
            .map_err(|e| DomainError::Validation(format!("Invalid rates JSON: {e}")))?;

        let rates: Vec<ExchangeRate> = inputs
            .into_iter()
            .map(|input| ExchangeRate {
                from_currency: input.from.to_uppercase(),
                to_currency: input.to.to_uppercase(),
                rate: (input.rate * RATE_PRECISION as f64).round() as i64,
                source: RateSource::Cached,
                fetched_at: Utc::now(),
            })
            .collect();

        let count = rates.len();
        self.rate_repo.save_batch(&rates)?;
        Ok(count)
    }

    /// Set a manual exchange rate (user override — highest priority).
    pub fn set_manual_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
        rate: f64,
    ) -> Result<ExchangeRate, DomainError> {
        if rate <= 0.0 {
            return Err(DomainError::Validation(
                "Exchange rate must be positive".to_string(),
            ));
        }

        let exchange_rate = ExchangeRate::new(
            from_currency.to_string(),
            to_currency.to_string(),
            (rate * RATE_PRECISION as f64).round() as i64,
            RateSource::UserOverride,
        );

        self.rate_repo.save(&exchange_rate)?;
        Ok(exchange_rate)
    }

    /// Convert an amount between currencies using the 3-tier rate resolution.
    pub fn convert(
        &self,
        amount_cents: i64,
        from_currency: &str,
        to_currency: &str,
    ) -> Result<ConversionResult, DomainError> {
        let rate = self
            .rate_repo
            .find_best_rate(from_currency, to_currency)?
            .ok_or_else(|| {
                DomainError::NotFound(format!(
                    "No exchange rate found for {} → {}",
                    from_currency.to_uppercase(),
                    to_currency.to_uppercase()
                ))
            })?;

        let converted = rate.convert(amount_cents);

        Ok(ConversionResult {
            original_amount: amount_cents,
            converted_amount: converted,
            from_currency: rate.from_currency.clone(),
            to_currency: rate.to_currency.clone(),
            rate: rate.rate,
            rate_source: rate.source,
            rate_fetched_at: rate.fetched_at,
        })
    }

    /// Get freshness info for a specific currency pair.
    pub fn get_rate_freshness(
        &self,
        from_currency: &str,
        to_currency: &str,
    ) -> Result<Option<RateFreshness>, DomainError> {
        let rate = self
            .rate_repo
            .find_best_rate(from_currency, to_currency)?;

        Ok(rate.map(|r| {
            let age = Utc::now()
                .signed_duration_since(r.fetched_at)
                .num_seconds();
            RateFreshness {
                source: r.source,
                fetched_at: r.fetched_at,
                age_seconds: age,
            }
        }))
    }

    /// List all available rates from a base currency.
    pub fn list_rates(
        &self,
        from_currency: &str,
    ) -> Result<Vec<ExchangeRate>, DomainError> {
        self.rate_repo.find_by_from_currency(from_currency)
    }
}

use crate::entities::exchange_rate::ExchangeRate;
use crate::errors::DomainError;

/// Repository trait for exchange rate persistence.
pub trait ExchangeRateRepository: Send + Sync {
    /// Save or update an exchange rate.
    fn save(&self, rate: &ExchangeRate) -> Result<(), DomainError>;

    /// Find the best rate for a currency pair using 3-tier resolution:
    /// user_override > cached > bundled.
    fn find_best_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
    ) -> Result<Option<ExchangeRate>, DomainError>;

    /// Find all rates for a given base currency.
    fn find_by_from_currency(
        &self,
        from_currency: &str,
    ) -> Result<Vec<ExchangeRate>, DomainError>;

    /// Batch-save multiple rates (used when updating from API).
    fn save_batch(&self, rates: &[ExchangeRate]) -> Result<(), DomainError>;

    /// Delete all rates of a given source type.
    fn delete_by_source(
        &self,
        source: &crate::entities::exchange_rate::RateSource,
    ) -> Result<(), DomainError>;
}

use chrono::{DateTime, Utc};
use rusqlite::params;

use finance_core::entities::exchange_rate::{ExchangeRate, RateSource};
use finance_core::errors::DomainError;
use finance_core::repositories::ExchangeRateRepository;

use crate::database::Database;
use crate::date_utils::format_dt;
use crate::error::StorageError;

/// SQLite implementation of ExchangeRateRepository.
pub struct SqliteExchangeRateRepository<'a> {
    db: &'a Database,
}

impl<'a> SqliteExchangeRateRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }
}

impl<'a> ExchangeRateRepository for SqliteExchangeRateRepository<'a> {
    fn save(&self, rate: &ExchangeRate) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO exchange_rates (from_currency, to_currency, rate, source, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                rate.from_currency,
                rate.to_currency,
                rate.rate,
                rate.source.to_string(),
                format_dt(&rate.fetched_at),
            ],
        )
        .map_err(StorageError::from)?;
        Ok(())
    }

    fn find_best_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
    ) -> Result<Option<ExchangeRate>, DomainError> {
        let from = from_currency.to_uppercase();
        let to = to_currency.to_uppercase();

        // Same currency → identity rate
        if from == to {
            return Ok(Some(ExchangeRate {
                from_currency: from.clone(),
                to_currency: to,
                rate: 1_000_000, // 1.0
                source: RateSource::Bundled,
                fetched_at: Utc::now(),
            }));
        }

        let conn = self.db.conn.lock().unwrap();

        // Query all rates for this pair, order by source priority DESC
        let mut stmt = conn
            .prepare(
                "SELECT from_currency, to_currency, rate, source, fetched_at
                 FROM exchange_rates
                 WHERE from_currency = ?1 AND to_currency = ?2
                 ORDER BY CASE source
                    WHEN 'user_override' THEN 2
                    WHEN 'cached' THEN 1
                    WHEN 'bundled' THEN 0
                 END DESC
                 LIMIT 1",
            )
            .map_err(StorageError::from)?;

        let result = stmt.query_row(params![from, to], |row| row_to_rate(row));

        match result {
            Ok(rate) => Ok(Some(rate)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::from(e).into()),
        }
    }

    fn find_by_from_currency(
        &self,
        from_currency: &str,
    ) -> Result<Vec<ExchangeRate>, DomainError> {
        let from = from_currency.to_uppercase();
        let conn = self.db.conn.lock().unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT from_currency, to_currency, rate, source, fetched_at
                 FROM exchange_rates
                 WHERE from_currency = ?1
                 ORDER BY to_currency, CASE source
                    WHEN 'user_override' THEN 2
                    WHEN 'cached' THEN 1
                    WHEN 'bundled' THEN 0
                 END DESC",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map(params![from], |row| row_to_rate(row))
            .map_err(StorageError::from)?;

        let mut rates = Vec::new();
        for row in rows {
            rates.push(row.map_err(StorageError::from)?);
        }
        Ok(rates)
    }

    fn save_batch(&self, rates: &[ExchangeRate]) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        let tx = conn
            .unchecked_transaction()
            .map_err(StorageError::from)?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO exchange_rates (from_currency, to_currency, rate, source, fetched_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                )
                .map_err(StorageError::from)?;

            for rate in rates {
                stmt.execute(params![
                    rate.from_currency,
                    rate.to_currency,
                    rate.rate,
                    rate.source.to_string(),
                    format_dt(&rate.fetched_at),
                ])
                .map_err(StorageError::from)?;
            }
        }

        tx.commit().map_err(StorageError::from)?;
        Ok(())
    }

    fn delete_by_source(&self, source: &RateSource) -> Result<(), DomainError> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM exchange_rates WHERE source = ?1",
            params![source.to_string()],
        )
        .map_err(StorageError::from)?;
        Ok(())
    }
}

fn row_to_rate(row: &rusqlite::Row) -> rusqlite::Result<ExchangeRate> {
    let source_str: String = row.get(3)?;
    let fetched_str: String = row.get(4)?;

    Ok(ExchangeRate {
        from_currency: row.get(0)?,
        to_currency: row.get(1)?,
        rate: row.get(2)?,
        source: RateSource::from_str(&source_str).unwrap_or(RateSource::Bundled),
        fetched_at: DateTime::parse_from_rfc3339(&fetched_str)
            .unwrap()
            .with_timezone(&Utc),
    })
}

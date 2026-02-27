use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::common::{BaseEntity, TransactionType};

/// How frequently a recurring transaction occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    BiWeekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl RecurrenceFrequency {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "daily" => Some(Self::Daily),
            "weekly" => Some(Self::Weekly),
            "biweekly" => Some(Self::BiWeekly),
            "monthly" => Some(Self::Monthly),
            "quarterly" => Some(Self::Quarterly),
            "yearly" => Some(Self::Yearly),
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::Daily => "daily".to_string(),
            Self::Weekly => "weekly".to_string(),
            Self::BiWeekly => "biweekly".to_string(),
            Self::Monthly => "monthly".to_string(),
            Self::Quarterly => "quarterly".to_string(),
            Self::Yearly => "yearly".to_string(),
        }
    }
}

/// A recurring transaction template that can spawn regular transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringTransaction {
    #[serde(flatten)]
    pub base: BaseEntity,
    pub account_id: Uuid,
    pub category_id: Uuid,
    pub amount: i64,
    pub transaction_type: TransactionType,
    pub description: String,
    pub frequency: RecurrenceFrequency,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub next_occurrence: DateTime<Utc>,
    pub is_active: bool,
}

impl RecurringTransaction {
    pub fn new(
        account_id: Uuid,
        category_id: Uuid,
        amount: i64,
        transaction_type: TransactionType,
        description: String,
        frequency: RecurrenceFrequency,
        start_date: DateTime<Utc>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<Self, String> {
        if amount <= 0 {
            return Err("Amount must be positive".to_string());
        }

        if let Some(end) = end_date {
            if end < start_date {
                return Err("End date must be after start date".to_string());
            }
        }

        Ok(Self {
            base: BaseEntity::new(),
            account_id,
            category_id,
            amount,
            transaction_type,
            description,
            frequency,
            start_date,
            end_date,
            next_occurrence: start_date,
            is_active: true,
        })
    }

    /// Check if this recurring transaction is still active (not expired).
    pub fn is_expired(&self) -> bool {
        if !self.is_active {
            return true;
        }

        if let Some(end) = self.end_date {
            Utc::now() > end
        } else {
            false
        }
    }

    /// Calculate the next occurrence date after the current next_occurrence.
    pub fn calculate_next_occurrence(&self) -> DateTime<Utc> {
        use chrono::Datelike;

        match self.frequency {
            RecurrenceFrequency::Daily => {
                self.next_occurrence + chrono::Duration::days(1)
            }
            RecurrenceFrequency::Weekly => {
                self.next_occurrence + chrono::Duration::weeks(1)
            }
            RecurrenceFrequency::BiWeekly => {
                self.next_occurrence + chrono::Duration::weeks(2)
            }
            RecurrenceFrequency::Monthly => {
                // Add one month (handle varying month lengths)
                let next = if self.next_occurrence.month() == 12 {
                    self.next_occurrence
                        .with_year(self.next_occurrence.year() + 1)
                        .unwrap()
                        .with_month(1)
                        .unwrap()
                } else {
                    self.next_occurrence
                        .with_month(self.next_occurrence.month() + 1)
                        .unwrap()
                };
                next
            }
            RecurrenceFrequency::Quarterly => {
                // Add 3 months
                let current_month = self.next_occurrence.month();
                let months_to_add = 3;
                let new_month = ((current_month - 1 + months_to_add) % 12) + 1;
                let year_offset = (current_month - 1 + months_to_add) / 12;

                self.next_occurrence
                    .with_year(self.next_occurrence.year() + year_offset as i32)
                    .unwrap()
                    .with_month(new_month)
                    .unwrap()
            }
            RecurrenceFrequency::Yearly => {
                self.next_occurrence
                    .with_year(self.next_occurrence.year() + 1)
                    .unwrap()
            }
        }
    }
}

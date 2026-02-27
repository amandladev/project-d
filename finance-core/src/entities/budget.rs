use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::common::BaseEntity;

/// Time period for a budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetPeriod {
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl BudgetPeriod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "weekly" => Some(Self::Weekly),
            "monthly" => Some(Self::Monthly),
            "quarterly" => Some(Self::Quarterly),
            "yearly" => Some(Self::Yearly),
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::Weekly => "weekly".to_string(),
            Self::Monthly => "monthly".to_string(),
            Self::Quarterly => "quarterly".to_string(),
            Self::Yearly => "yearly".to_string(),
        }
    }
}

/// A budget for tracking spending limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    #[serde(flatten)]
    pub base: BaseEntity,
    pub account_id: Uuid,
    pub category_id: Option<Uuid>,
    pub name: String,
    pub amount: i64,
    pub period: BudgetPeriod,
    pub start_date: DateTime<Utc>,
}

impl Budget {
    pub fn new(
        account_id: Uuid,
        category_id: Option<Uuid>,
        name: String,
        amount: i64,
        period: BudgetPeriod,
        start_date: DateTime<Utc>,
    ) -> Result<Self, String> {
        if amount <= 0 {
            return Err("Budget amount must be positive".to_string());
        }

        if name.trim().is_empty() {
            return Err("Budget name cannot be empty".to_string());
        }

        Ok(Self {
            base: BaseEntity::new(),
            account_id,
            category_id,
            name,
            amount,
            period,
            start_date,
        })
    }

    /// Calculate the end date of the current budget period.
    pub fn period_end_date(&self) -> DateTime<Utc> {
        use chrono::Datelike;

        match self.period {
            BudgetPeriod::Weekly => self.start_date + chrono::Duration::weeks(1),
            BudgetPeriod::Monthly => {
                if self.start_date.month() == 12 {
                    self.start_date
                        .with_year(self.start_date.year() + 1)
                        .unwrap()
                        .with_month(1)
                        .unwrap()
                } else {
                    self.start_date
                        .with_month(self.start_date.month() + 1)
                        .unwrap()
                }
            }
            BudgetPeriod::Quarterly => {
                let current_month = self.start_date.month();
                let new_month = ((current_month - 1 + 3) % 12) + 1;
                let year_offset = (current_month - 1 + 3) / 12;

                self.start_date
                    .with_year(self.start_date.year() + year_offset as i32)
                    .unwrap()
                    .with_month(new_month)
                    .unwrap()
            }
            BudgetPeriod::Yearly => self.start_date.with_year(self.start_date.year() + 1).unwrap(),
        }
    }
}

/// Budget progress tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetProgress {
    pub budget: Budget,
    pub spent: i64,
    pub remaining: i64,
    pub percentage: f64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

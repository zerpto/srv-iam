use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The API is up (`Health` in the API contract).
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct Health {
    pub status: HealthStatus,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ok,
}

impl Health {
    #[must_use]
    pub const fn ok() -> Self {
        Self {
            status: HealthStatus::Ok,
        }
    }
}

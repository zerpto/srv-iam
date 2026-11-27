use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS)]
pub struct VerifyEmailMeta {
    pub web_urls: Vec<String>,
}

pub use failure::Error;
pub use serde_derive::{Deserialize, Serialize};
pub use uuid::Uuid;

pub use mediavault_common::types::DateTime;

pub fn now() -> DateTime {
    chrono::Utc::now()
}

pub fn uuid() -> Uuid {
    Uuid::new_v4()
}

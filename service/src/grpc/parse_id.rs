use tonic::Status;
use uuid::Uuid;

/// Reads a plain UUID from a request field.
///
/// # Errors
///
/// `invalid_argument` naming `field` when `value` is not a UUID.
pub fn parse_id(field: &str, value: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(value.trim())
        .map_err(|_| Status::invalid_argument(format!("{field} must be a UUID")))
}

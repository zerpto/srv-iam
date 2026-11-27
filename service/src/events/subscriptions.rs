use zerpto_core::events::consumer::Subscription;

/// The events this service consumes. Empty for now; add one entry per
/// subscription, for example:
///
/// ```ignore
/// Subscription {
///     stream: "BILLING".to_string(),
///     durable: "inventory-invoice-paid".to_string(),
///     filter_subject: "zerpto.billing.invoice.paid".to_string(),
///     handler: std::sync::Arc::new(InvoicePaidHandler),
/// }
/// ```
#[must_use]
pub const fn subscriptions() -> Vec<Subscription> {
    Vec::new()
}

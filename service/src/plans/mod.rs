//! Where a tenant's plan comes from.
//!
//! Community code asks the registered [`PlanProvider`](provider::PlanProvider) and
//! never knows who answers. The community edition registers
//! [`Unlimited`](unlimited::Unlimited); a service that extends this one replaces it
//! in `after_context`. The rules that apply a plan are in `models::plan`;
//! [`decide`](decide::decide) combines them with a member's permissions.

pub mod decide;
pub mod provider;
pub mod unlimited;

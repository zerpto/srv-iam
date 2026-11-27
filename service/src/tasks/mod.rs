//! Loco tasks (`cargo loco task <name>`), registered in `App::register_tasks`.
//!
//! A task can also run on a schedule: see `scheduler:` in `config/*.yaml`.
//! `outbox:cleanup` comes from `zerpto_core::events::cleanup`.

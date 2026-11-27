use axum::middleware::from_fn;
use loco_rs::prelude::*;
use zerpto_core::http::problem::with_request_id;

use super::controllers::{
    auth, extensions, health, invitation, menu, permissions, reset_password, roles, tenants,
    user_permissions, users, verify,
};

/// Every `/v1` route group of this service, mounted in `App::routes` with
/// `AppRoutes::add_routes`.
///
/// A group has one prefix, so each controller gets its own `Routes` here, with the
/// request-id layer so errors quote it.
pub fn routes() -> Vec<Routes> {
    vec![
        Routes::new()
            .prefix("/v1/health")
            .add("/", get(health::health))
            .layer(from_fn(with_request_id)),
        Routes::new()
            .prefix("/v1/auth")
            .add("/sign-up", post(auth::sign_up))
            .add("/verify-email", post(verify::verify_email))
            .add("/sign-in", post(auth::sign_in))
            .add("/google", post(verify::google))
            .add("/refresh", post(auth::refresh))
            .add("/sign-out", delete(auth::sign_out))
            .add("/password", put(reset_password::change_password))
            .add(
                "/reset-password",
                post(reset_password::forgot_password).put(reset_password::reset_password),
            )
            .add("/accept-invite", post(invitation::accept_invite))
            .add(
                "/accept-invite/existing",
                post(invitation::accept_invite_existing),
            )
            .add("/phone/code", post(verify::send_phone_code))
            .add("/phone/verify", post(verify::verify_phone_code))
            .add("/me", get(auth::me))
            .add("/switch-tenant", post(tenants::switch_tenant))
            .layer(from_fn(with_request_id)),
        Routes::new()
            .prefix("/v1/iam/users")
            .add("/", get(users::list).post(users::create))
            .add(
                "/{id}",
                get(users::show).patch(users::update).delete(users::revoke),
            )
            .add(
                "/{id}/activate",
                put(users::reactivate).delete(users::deactivate),
            )
            .add(
                "/{id}/permissions",
                get(user_permissions::show).put(user_permissions::replace),
            )
            .layer(from_fn(with_request_id)),
        Routes::new()
            .prefix("/v1/iam/roles")
            .add("/", get(roles::list).post(roles::create))
            .add(
                "/{id}",
                get(roles::show).patch(roles::update).delete(roles::remove),
            )
            .layer(from_fn(with_request_id)),
        Routes::new()
            .prefix("/v1/iam/tenant")
            .add("/", get(tenants::show).patch(tenants::update))
            .layer(from_fn(with_request_id)),
        Routes::new()
            .prefix("/v1/iam/tenants")
            .add("/", get(tenants::index).post(tenants::create))
            .layer(from_fn(with_request_id)),
        Routes::new()
            .prefix("/v1/iam/permissions")
            .add("/", get(permissions::list))
            .layer(from_fn(with_request_id)),
        Routes::new()
            .prefix("/v1/iam/extensions")
            .add("/", get(extensions::list).post(extensions::register))
            .add("/{id}", get(extensions::show))
            .add("/{id}/refresh", post(extensions::refresh))
            .add("/{id}/approve", post(extensions::approve))
            .add("/{id}/reject", post(extensions::reject))
            .add("/{id}/enable", post(extensions::enable))
            .add("/{id}/disable", post(extensions::disable))
            .layer(from_fn(with_request_id)),
        Routes::new()
            .prefix("/v1/iam/menu")
            .add("/", get(menu::show))
            .layer(from_fn(with_request_id)),
    ]
}

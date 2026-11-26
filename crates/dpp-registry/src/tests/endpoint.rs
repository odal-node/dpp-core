//! The registry endpoints, their hosts and token routes.

use super::super::*;
use chrono::Utc;

#[test]
fn sandbox_endpoint() {
    let ep = RegistryEndpoint::sandbox();
    assert_eq!(ep.authority, RegistryAuthority::EuSandbox);
    assert!(!ep.mtls_required);
    // The Commission's test environment is the `acc` sibling of the production
    // host, not a "sandbox"-named one. This asserted `contains("sandbox")` while
    // the URL was invented, which is precisely the shape of test that agrees
    // with our own guess instead of an outside fact.
    assert_eq!(
        ep.base_url,
        "https://reg.acc.product-passport.ec.europa.eu/api/v1"
    );
    assert_eq!(
        ep.dashboard_base_url,
        "https://reg.acc.product-passport.ec.europa.eu/dashboard/api/v1"
    );
}

/// 🚨 The API is not on the host the Commission publishes for the registry.
///
/// `registry.…` serves the web interface. The interface's own runtime
/// configuration (read 2026-10-14) sends its API calls to `reg.…`, and this
/// crate pointed every request at the interface's host until then.
#[test]
fn the_api_is_called_on_the_hosts_the_registry_interface_calls() {
    for ep in [RegistryEndpoint::sandbox(), RegistryEndpoint::production()] {
        for url in [&ep.base_url, &ep.dashboard_base_url] {
            assert!(
                url.starts_with("https://reg."),
                "{url} is not on the API host"
            );
            assert!(!url.contains("://registry."), "{url} is the interface host");
        }
        let token = ep.token_endpoint.as_deref().expect("a token URL");
        assert!(token.starts_with("https://auth."), "{token}");
    }
}

/// The two environments must never collapse onto one host — submitting test
/// data to the operational registry is not a recoverable mistake.
#[test]
fn sandbox_and_production_are_different_hosts() {
    let sandbox = RegistryEndpoint::sandbox();
    let production = RegistryEndpoint::production();
    assert_ne!(sandbox.base_url, production.base_url);
    assert!(
        sandbox.base_url.contains(".acc."),
        "the test environment is the `acc` host: {}",
        sandbox.base_url
    );
    assert!(
        !production.base_url.contains(".acc."),
        "production must not point at the test environment: {}",
        production.base_url
    );
}

#[test]
fn production_endpoint() {
    let ep = RegistryEndpoint::production();
    assert_eq!(ep.authority, RegistryAuthority::EuCentral);
    assert!(ep.mtls_required);
}

/// Every endpoint's token URL is the declared route, so the basis the table
/// gives `TOKEN_PATH` is the basis of the URL a caller actually calls.
#[test]
fn every_token_endpoint_is_the_declared_token_route() {
    for ep in [
        RegistryEndpoint::sandbox(),
        RegistryEndpoint::production(),
        RegistryEndpoint::stand_in("http://127.0.0.1:8090"),
    ] {
        let token = ep.token_endpoint.expect("each endpoint names a token URL");
        assert!(
            token.ends_with(TOKEN_PATH),
            "{token} is not the declared token route"
        );
        // Beside the API, not under it.
        assert!(!token.contains("/api/v1"), "{token}");
    }
}

/// A stand-in is shaped like the registry's endpoints, so a caller takes the
/// same path against it, and is named as what it is, so nothing mistakes it for
/// either of them.
#[test]
fn a_stand_in_endpoint_mirrors_the_registry_shape_under_its_own_name() {
    let ep = RegistryEndpoint::stand_in("http://127.0.0.1:8090/");
    assert_eq!(ep.authority, RegistryAuthority::StandIn);
    assert_eq!(ep.base_url, "http://127.0.0.1:8090/api/v1");
    assert_eq!(
        ep.dashboard_base_url,
        "http://127.0.0.1:8090/dashboard/api/v1"
    );
    assert_eq!(
        ep.token_endpoint.as_deref(),
        Some("http://127.0.0.1:8090/oauth2/token")
    );
    assert_ne!(ep.authority, RegistryEndpoint::sandbox().authority);
    assert_ne!(ep.authority, RegistryEndpoint::production().authority);
    assert_eq!(
        serde_json::to_value(&ep.authority).unwrap(),
        "stand_in",
        "a stored configuration names the stand-in as one"
    );
}

#[test]
fn status_response_round_trip() {
    let status = StatusResponse {
        registry_id: "EU-REG-2026-00001".into(),
        status: RegistryStatusCode::Registered,
        updated_at: Utc::now(),
        message: None,
    };
    let json = serde_json::to_string(&status).unwrap();
    let back: StatusResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(back.status, RegistryStatusCode::Registered);
}

// ── B1 validation tests ─────────────────────────────────────────────────

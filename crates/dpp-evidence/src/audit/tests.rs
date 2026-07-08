use super::*;
use chrono::Utc;
use uuid::Uuid;

fn entry(action: &str) -> AuditEntry {
    AuditEntry {
        id: Uuid::now_v7(),
        passport_id: "p1".into(),
        actor: "actor".into(),
        action: action.into(),
        previous_status: None,
        new_status: None,
        metadata: None,
        timestamp: Utc::now(),
        prev_hash: None,
        entry_hash: None,
    }
}

/// Link a slice into a chain exactly as the engine's storage layer does on append.
fn chain(entries: &mut [AuditEntry]) {
    let mut prev = GENESIS_PREV_HASH.to_owned();
    for e in entries.iter_mut() {
        let h = e.chain_hash(&prev);
        e.prev_hash = Some(prev.clone());
        e.entry_hash = Some(h.clone());
        prev = h;
    }
}

#[test]
fn chain_hash_is_deterministic_and_prev_sensitive() {
    let e = entry("created");
    assert_eq!(e.chain_hash(""), e.chain_hash(""));
    assert_ne!(e.chain_hash(""), e.chain_hash("deadbeef"));
}

#[test]
fn new_builds_from_a_plain_actor_string() {
    let e = AuditEntry::new("p1", "created", "user-123", None, Some("draft"));
    assert_eq!(e.actor, "user-123");
    assert_eq!(e.action, "created");
    assert_eq!(e.new_status.as_deref(), Some("draft"));
}

#[test]
fn intact_chain_verifies() {
    let mut es = [entry("created"), entry("published"), entry("suspended")];
    chain(&mut es);
    assert!(verify_audit_chain(&es).is_ok());
}

#[test]
fn tampered_content_breaks_at_exact_index() {
    let mut es = [entry("created"), entry("published"), entry("archived")];
    chain(&mut es);
    es[1].new_status = Some("suspended".into()); // flip content, keep stored hash
    let brk = verify_audit_chain(&es).expect_err("tamper must be detected");
    assert_eq!(brk.index, 1);
    assert!(brk.reason.contains("tampered"));
}

#[test]
fn broken_prev_link_detected() {
    let mut es = [entry("created"), entry("published")];
    chain(&mut es);
    es[1].prev_hash = Some("0000".into());
    assert_eq!(verify_audit_chain(&es).expect_err("break").index, 1);
}

#[test]
fn unknown_field_is_rejected_at_deserialize() {
    let mut value = serde_json::to_value(entry("created")).unwrap();
    value["notARealField"] = serde_json::json!("sneaky");
    let result: Result<AuditEntry, _> = serde_json::from_value(value);
    assert!(result.is_err(), "unknown field must fail to deserialize");
}

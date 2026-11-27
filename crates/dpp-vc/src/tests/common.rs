//! Helpers the identity tests share.

use std::future::Future;

/// Whether `jws` resolves in `doc` to a key it verifies under.
pub(super) fn verifies_in(doc: &serde_json::Value, jws: &str) -> bool {
    dpp_crypto::jws::verifier::resolve_verification_key(doc, jws)
        .is_some_and(|key| dpp_crypto::jws::verifier::verify_jws(jws, &key).is_ok())
}

/// The `kid` a compact JWS names, as a verifier reads it.
pub(super) fn kid_named(jws: &str) -> String {
    dpp_crypto::jws::verifier::extract_kid_from_jws(jws).expect("the token names a kid")
}

/// The output of a future that awaits nothing, as `sign_passport`'s does: it signs
/// in process. Polled once on this thread, so many threads can sign at once without
/// a runtime each.
pub(super) fn without_waiting<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    match future.as_mut().poll(&mut context) {
        std::task::Poll::Ready(output) => output,
        std::task::Poll::Pending => panic!("the future waited"),
    }
}

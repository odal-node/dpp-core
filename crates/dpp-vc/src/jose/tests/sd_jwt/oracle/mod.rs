//! The `vc+sd-jwt` and `vp+sd-jwt` tokens this crate issues, for two independent
//! implementations of RFC 9901 to judge.
//!
//! ```text
//! EMIT_VC_SD_JWT_CORPUS=1 cargo test -p dpp-vc --lib jose::tests::sd_jwt::oracle
//! ```
//!
//! writes `target/sdjwt-oracle/vc-sd-jwt.json`, which `.github/oracle/sdjwt/vc-sd-jwt.mjs`
//! (the TypeScript library) and `VcSdJwtOracle.kt` (the EU wallet's Kotlin library) read.
//! Each case carries the token, the public key it was signed under, the document it was
//! made from, the paths that were concealed, the claims a verifier must read from the
//! token as presented, and what this crate reads. What the two libraries judge: the
//! signature under the published key, the Disclosures and their digests, `typ` and `cty`,
//! and the processed payload against the document.
//!
//! The cases include the three issuance inputs of the W3C suite (rows 17 to 19), issued
//! under the suite's own keys, so what the suite asks this crate to issue is what the
//! other implementations read.
//!
//! It also holds presentations bound to a Holder's key: the signer writes the key as `cnf`, the
//! Holder binds a presentation with a Key Binding JWT carrying a nonce, an audience and the
//! `sd_hash`, and each is judged with Key Binding **required**, at the challenge the case
//! names. The controls are accepted by this crate and by both libraries. The negative controls
//! each break one rule, and each must be refused by all three: a wrong nonce, a wrong audience,
//! an `sd_hash` that belongs to another presentation, and a token with no Key Binding JWT.
//!
//! Without the variable the test still runs, and still checks that this crate reads every
//! token back to the claims it expects, so an ordinary `just check` gets that coverage
//! without producing files.

mod bound;
mod corpus;

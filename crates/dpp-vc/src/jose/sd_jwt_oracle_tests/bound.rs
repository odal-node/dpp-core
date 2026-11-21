//! The presentations bound to a Holder's key, for the oracle corpus.
//!
//! A child of `sd_jwt_oracle_tests`, which holds the cases that are issued and presented
//! whole. These are issued to a Holder, bound by it, and judged with Key Binding required.

use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::sd_jwt::{KeyBinding, KeyBindingPolicy, SdJwt};

use super::Issued;
use crate::test_support::{KID, credential, generated, presentation};

/// The challenge a bound case answers: the Verifier, the nonce it issued, the instant it
/// verifies at, and how far `iat` may lie from that instant either way, in seconds. One
/// number for both directions, because one of the libraries takes its window as a point and a
/// symmetric skew.
pub(super) const AUDIENCE: &str = "https://verifier.example";
pub(super) const NONCE: &str = "n-0S6_WzA2Mj";
pub(super) const NOW: i64 = 1_800_000_000;
pub(super) const WINDOW: u32 = 300;

pub(super) fn policy() -> KeyBindingPolicy {
    KeyBindingPolicy {
        audience: AUDIENCE.to_owned(),
        nonce: NONCE.to_owned(),
        now: NOW,
        max_age: WINDOW,
        leeway: WINDOW,
    }
}

fn binding() -> KeyBinding {
    KeyBinding {
        audience: AUDIENCE.to_owned(),
        nonce: NONCE.to_owned(),
        issued_at: NOW,
    }
}

impl Issued {
    /// A case judged with Key Binding required. `accepts` is the verdict every reader must
    /// reach, and `expected` the claims it must read when that is acceptance.
    fn bound_case(&self, label: &str, token: &SdJwt, expected: Option<&Value>) -> Value {
        let mut case = self.case_of(label, token, expected.unwrap_or(&Value::Null), true);
        case["accepts"] = json!(expected.is_some());
        case["challenge"] = json!({
            "nonce": NONCE,
            "aud": AUDIENCE,
            "now": NOW,
            "window": WINDOW,
        });
        case
    }

    /// The token bound by the Holder to `binding`, with every Disclosure presented.
    fn bound(&self, binding: &KeyBinding) -> SdJwt {
        let holder = self.holder.as_ref().expect("issued to a Holder");
        let digests: Vec<String> = self
            .sd_jwt
            .disclosures()
            .iter()
            .map(|d| d.digest())
            .collect();
        let reveal: Vec<&str> = digests.iter().map(String::as_str).collect();
        self.sd_jwt
            .present_with_key_binding(&reveal, binding, holder)
            .expect("the Holder binds")
    }

    /// The bound token and the four that each break one rule of Key Binding.
    pub(super) fn bound_cases(&self) -> Vec<Value> {
        let holder = self.holder.as_ref().expect("issued to a Holder");
        let valid = self.bound(&binding());
        let mut cases = vec![self.bound_case("valid", &valid, Some(&self.document))];
        if !self.id.starts_with("bound/vc-ed25519") {
            return cases;
        }
        let wrong_nonce = self.bound(&KeyBinding {
            nonce: "another-nonce".to_owned(),
            ..binding()
        });
        let wrong_audience = self.bound(&KeyBinding {
            audience: "https://elsewhere.example".to_owned(),
            ..binding()
        });
        // A Key Binding JWT made for a presentation that disclosed nothing, attached to one
        // that discloses everything: its `sd_hash` is another token's.
        let withheld = self
            .sd_jwt
            .present_with_key_binding(&[], &binding(), holder)
            .expect("the Holder binds")
            .serialise();
        let key_binding_jwt = withheld.rsplit('~').next().expect("a Key Binding JWT");
        let wrong_sd_hash =
            SdJwt::parse(&format!("{}{key_binding_jwt}", self.sd_jwt.serialise())).expect("parses");
        cases.push(self.bound_case("wrong-nonce", &wrong_nonce, None));
        cases.push(self.bound_case("wrong-audience", &wrong_audience, None));
        cases.push(self.bound_case("wrong-sd-hash", &wrong_sd_hash, None));
        cases.push(self.bound_case("no-key-binding-jwt", &self.sd_jwt, None));
        cases
    }
}

/// What is issued to a Holder: the Holder's key is in `cnf`, a verifier requires Key Binding,
/// and the Holder binds the presentation.
pub(super) fn issued_to_holders() -> Vec<Issued> {
    vec![
        Issued::issue(
            "bound/vc-ed25519",
            false,
            generated(KeyAlgorithm::EcdsaP256),
            KID.to_owned(),
            credential(),
            &json!([["credentialSubject", "name"]]),
            Some(generated(KeyAlgorithm::Ed25519)),
        ),
        Issued::issue(
            "bound/vc-p256",
            false,
            generated(KeyAlgorithm::Ed25519),
            KID.to_owned(),
            credential(),
            &json!([["credentialSubject", "name"]]),
            Some(generated(KeyAlgorithm::EcdsaP256)),
        ),
        Issued::issue(
            "bound/vp-ed25519",
            true,
            generated(KeyAlgorithm::EcdsaP256),
            KID.to_owned(),
            presentation(vec![]),
            &json!([["holder"]]),
            Some(generated(KeyAlgorithm::Ed25519)),
        ),
    ]
}

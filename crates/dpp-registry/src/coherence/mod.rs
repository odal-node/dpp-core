//! Whether a registration agrees with the passport it registers.
//!
//! # What the registry checks that no field check can
//!
//! [`RegistrationPayload::validate`](crate::RegistrationPayload::validate)
//! asks whether a registration is well formed on its own. IR (EU) 2026/1778
//! **Art. 8(7)** has the registry ask a second question on submission: it
//! reads the passport itself and confirms that the registration data cohere
//! with it (point (b)) and that the passport conforms to the level it is
//! registered at (point (c)). A registration can be perfectly formed and still
//! name a model, a batch or a carrier the passport contradicts.
//!
//! [`RegistrationPayload::check_coherence`](crate::RegistrationPayload::check_coherence)
//! asks that second question here, so that a consumer finds a contradiction
//! before the registry does, and so that a stand-in exercising the same check
//! reads the article from this crate rather than writing its own reading of it.
//!
//! # What it cannot know
//!
//! Which data are *"mandatory data to be uploaded in the registry"* is for the
//! applicable delegated act and the registry's own templates, neither of which
//! is published for any product group. The check compares the fields this
//! crate registers and the passport also states. That is a reading of
//! point (b), not a transcription of the registry's rule set.
//!
//! Tested in `dpp-tests` (`registry_coherence`), against passports built the
//! way a consumer builds them: this crate has no passport fixtures of its own.

mod check;

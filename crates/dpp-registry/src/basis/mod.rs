//! [`RegistryBasis`] — what this crate knows a wire detail *from*.
//!
//! # Why this is a type and not a doc comment
//!
//! Every endpoint path, header and status code in this crate already says in
//! prose whether it was observed somewhere or invented to fill a gap. That is
//! the right thing to record and the wrong place to keep it: a doc comment
//! cannot be read by the code that acts on it.
//!
//! Three things need to read it, and none of them can today.
//!
//! - **A consumer standing a mock registry up against these types.** A mock
//!   built on this crate is self-consistent by construction — it agrees with the
//!   client because both come from the same source — and that is genuinely
//!   useful for catching serialisation drift and state-machine mistakes. It is
//!   not evidence of registry conformance, and a green run looks identical
//!   either way. A mock that can name the basis of each route it serves can say
//!   which of the two it just demonstrated.
//! - **A test claiming conformance.** "This exercises no invented route" is
//!   currently unassertable, and therefore currently untrue by accident.
//! - **Anything that ages an observation.** The registry's User Guide changed a
//!   stated identifier limit by a factor of forty inside a month. An observation
//!   has a shelf life; a date in prose does not measure it.
//!
//! # The vocabulary is the workspace's, on purpose
//!
//! `Observed`/`Assumed` is the distinction `CitationBasis` draws for a citation
//! reason and `ParameterBasis` draws for a calculation input: *did anyone go and
//! look, or is this written from what we expected to find*. A bespoke marker
//! here would be understood only by the rule that reads it.
//!
//! # And the default runs the cautious way
//!
//! There is no `Default`. A basis is stated at every site or the site does not
//! compile, because the failure this exists to prevent is an unbacked value that
//! nobody marked — which is exactly what a default would reintroduce.

mod tier;

pub use tier::RegistryBasis;

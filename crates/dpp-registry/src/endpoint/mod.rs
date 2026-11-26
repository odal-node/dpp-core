//! [`RegistryEndpoint`] configuration, [`RegistryAuthority`], and the resource
//! paths beneath a base URL.
//!
//! # Where these paths come from, and what that is worth
//!
//! There is no published API specification. What exists is the registry's own
//! **unauthenticated web client**, which ships its endpoints as constants, and
//! reading it is how the `/api/v1` prefix was first learned. Re-read
//! **2026-09-16**, it also named the registration endpoint. Re-read
//! **2026-10-14**, together with the runtime configuration the client loads
//! (`assets/env-json-config.json`), it names the hosts the API is served from —
//! which are not the host the web interface is served from — and the dashboard
//! service a submission's outcome is read back from. Re-read **2026-10-20**, in
//! both the acceptance and the production build of the client, it showed three
//! things: the client's whole API surface is twenty-one calls and **none of them
//! transfers responsibility or reads a record's status**; the idempotency header
//! is set by one interceptor and nothing else; and the dashboard sorts by
//! `field,direction`, newest first by default.
//!
//! That is better evidence than an invention and it is **not a specification**.
//! It is one consumer's behaviour on one date, and a client can change without
//! notice — the registry's own User Guide changed a stated identifier limit by a
//! factor of forty between two versions in under a month. So each constant below
//! says which of the two it is, and nothing here should be read as *the registry
//! requires*.

mod routes;

pub use routes::*;

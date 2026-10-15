//! EU 2023/1542 Art. 8 recycled content, as this plugin reads it off a payload.
//!
//! The rules themselves live in `dpp-rules`: which phase binds a battery, the
//! minimum share per metal, and which metals a chemistry contains. What is here
//! is the plugin's half: which fields declare the shares, and the advisory
//! findings it reports about them.

mod findings;
mod second_life;
mod shares;

pub(crate) use findings::recycled_content_findings;
pub(crate) use shares::declared_shares;

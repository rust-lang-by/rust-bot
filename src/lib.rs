// Forbid panic-on-unexpected in production code: runtime failures must be
// modelled as errors (AppError / Result), not `unwrap`/`expect`/`panic!`. The
// few sanctioned panic sites (compile-time-constant regexes, infallible serde)
// carry a narrow `#[allow(clippy::expect_used)]`. Test code is exempt.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod bf_mention_handler;
pub mod boot;
pub mod chat_gpt_handler;
pub mod chat_repository;
pub mod error;
pub mod gayness_handler;
pub mod gpt_service;
pub mod mention_repository;
pub mod rust_mention_handler;
pub mod url_summary_handler;

pub use boot::{
    build_handler, message_has_url, run, AppDeps, GptParameters, MentionParameters,
    DEFAULT_OPENAI_BASE_URL,
};
pub use error::AppError;

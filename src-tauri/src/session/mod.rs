pub mod cli;
pub mod events;
pub mod manager;
pub mod projection;

pub use cli::ClaudeCli;
pub use events::{SessionEvent, SessionEventPayload};
pub use manager::SessionManager;

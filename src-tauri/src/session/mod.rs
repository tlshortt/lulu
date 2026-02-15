pub mod cli;
pub mod events;
pub mod manager;
pub mod projection;
pub mod supervisor;
pub mod worktree;

pub use cli::ClaudeCli;
pub use events::{SessionEvent, SessionEventPayload};
pub use manager::SessionManager;
pub use supervisor::{SessionRuntime, SessionSupervisor};
pub use worktree::WorktreeService;

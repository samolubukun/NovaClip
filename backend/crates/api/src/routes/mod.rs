pub mod health;
pub mod tasks;
pub mod clips;
pub mod media;
pub mod ai_edit;
pub mod youtube_studio;
pub mod mcp;

pub use health::health_router;
pub use tasks::tasks_router;
pub use clips::clips_router;
pub use media::media_router;
pub use ai_edit::ai_edit_router;
pub use youtube_studio::youtube_router;
pub use mcp::mcp_router;

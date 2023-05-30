#![warn(missing_docs)]
#![feature(drain_filter)]

//! The Scarab Engine Library
///
/// more documentation coming soon

/// The trait for running an app at a high level
mod app;
/// Common error and result types
pub mod error;
/// Game objects
pub mod gameobject;
/// Player input
pub mod input;
/// Rendering everything
pub mod rendering;
/// The scene wrapping game objects
pub mod scene;
/// Generic types
pub mod types;

pub use app::App;
pub use error::{PhysicsError, PhysicsResult, ScarabError, ScarabResult};
pub use glutin_window::GlutinWindow;
pub use winit::dpi::LogicalSize;

pub use scarab_macros::*;

pub mod aur;
pub mod config;
pub mod core;
pub mod flatpak;
pub mod gpg;
pub mod hooks;
pub mod pacman;
pub mod utils;

pub use crate::aur::get_deps;
pub use crate::core::{SearchResult, Source, install_with_priority, unified_search};

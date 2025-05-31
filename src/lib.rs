pub mod core;
pub mod aur;
pub mod pacman;
pub mod flatpak;
pub mod config;
pub mod utils;
pub mod hooks;
pub mod gpg;

pub use crate::core::{unified_search, install_with_priority, SearchResult, Source};
pub use crate::aur::get_deps;
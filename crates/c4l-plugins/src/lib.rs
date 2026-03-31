//! Plugin, skill, hook, and memory system for claw4love.
//!
//! Maps from:
//! - leak-claude-code: src/services/plugins/, src/skills/, src/hooks/
//! - ECC: 30 agents, 136 skills, 29 hooks
//! - Superpowers: 14 skills with YAML frontmatter format

pub mod skill;
pub mod hook;
pub mod memory;
pub mod plugin;

pub use skill::{SkillManifest, discover_skills, parse_skill_file};
pub use hook::{HookDef, HookEvent, HookResult, execute_hooks, load_hooks};
pub use memory::{MemoryFile, MemoryScope, load_memory_files};
pub use plugin::{PluginManifest, discover_plugins};

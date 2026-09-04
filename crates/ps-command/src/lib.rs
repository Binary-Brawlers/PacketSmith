//! Command registry, keybindings, and command palette integration for PacketSmith.
//!
//! Provides namespaced command identifiers, platform-aware keyboard shortcut mapping,
//! action dispatching, and command search for the desktop UI and CLI.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Command '{0}' not found in registry")]
    NotFound(String),
    #[error("Command '{0}' is currently disabled")]
    Disabled(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

/// Strongly-typed namespaced command identifier (e.g., "request.send", "tab.close").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandId(pub String);

impl CommandId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for CommandId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Keyboard modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Modifiers {
    pub primary: bool, // Cmd on macOS, Ctrl on Linux/Windows
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool, // Explicit Ctrl on macOS
}

/// Keyboard shortcut representation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyBinding {
    pub modifiers: Modifiers,
    pub key: String,
}

impl KeyBinding {
    pub fn primary(key: impl Into<String>) -> Self {
        Self {
            modifiers: Modifiers {
                primary: true,
                ..Default::default()
            },
            key: key.into(),
        }
    }

    pub fn primary_shift(key: impl Into<String>) -> Self {
        Self {
            modifiers: Modifiers {
                primary: true,
                shift: true,
                ..Default::default()
            },
            key: key.into(),
        }
    }

    /// Formats the keybinding for visual display in menus or tooltips.
    pub fn display_string(&self) -> String {
        let mut parts = Vec::new();
        if cfg!(target_os = "macos") {
            if self.modifiers.ctrl {
                parts.push("⌃");
            }
            if self.modifiers.alt {
                parts.push("⌥");
            }
            if self.modifiers.shift {
                parts.push("⇧");
            }
            if self.modifiers.primary {
                parts.push("⌘");
            }
        } else {
            if self.modifiers.primary || self.modifiers.ctrl {
                parts.push("Ctrl");
            }
            if self.modifiers.alt {
                parts.push("Alt");
            }
            if self.modifiers.shift {
                parts.push("Shift");
            }
        }
        parts.push(&self.key);
        parts.join("+")
    }
}

/// Metadata describing a command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandDescriptor {
    pub id: CommandId,
    pub title: String,
    pub category: String,
    pub description: Option<String>,
    pub default_keybinding: Option<KeyBinding>,
    pub enabled: bool,
}

impl CommandDescriptor {
    pub fn new(id: impl Into<CommandId>, title: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            category: category.into(),
            description: None,
            default_keybinding: None,
            enabled: true,
        }
    }

    pub fn with_keybinding(mut self, keybinding: KeyBinding) -> Self {
        self.default_keybinding = Some(keybinding);
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Action execution callback trait.
pub trait CommandAction: Send + Sync {
    fn execute(&self) -> Result<(), CommandError>;
}

impl<F> CommandAction for F
where
    F: Fn() -> Result<(), CommandError> + Send + Sync,
{
    fn execute(&self) -> Result<(), CommandError> {
        (self)()
    }
}

struct RegisteredCommand {
    descriptor: CommandDescriptor,
    action: Option<Arc<dyn CommandAction>>,
}

impl fmt::Debug for RegisteredCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RegisteredCommand")
            .field("descriptor", &self.descriptor)
            .field("has_action", &self.action.is_some())
            .finish()
    }
}

/// Central registry managing command lookup, palette searching, and execution.
#[derive(Debug, Default)]
pub struct CommandRegistry {
    commands: HashMap<CommandId, RegisteredCommand>,
    keymap: HashMap<KeyBinding, CommandId>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_standard_commands();
        registry
    }

    /// Registers standard platform commands (Request, View, Window, Workspace).
    fn register_standard_commands(&mut self) {
        self.register_descriptor(
            CommandDescriptor::new("request.send", "Send Request", "Request")
                .with_description("Execute the active HTTP or protocol request")
                .with_keybinding(KeyBinding::primary("Enter")),
        );

        self.register_descriptor(
            CommandDescriptor::new("request.cancel", "Cancel Request", "Request")
                .with_description("Cancel current in-flight request execution")
                .with_keybinding(KeyBinding::primary("Escape")),
        );

        self.register_descriptor(
            CommandDescriptor::new("tab.close", "Close Tab", "View")
                .with_description("Close the active workbench tab")
                .with_keybinding(KeyBinding::primary("w")),
        );

        self.register_descriptor(
            CommandDescriptor::new("workbench.command_palette", "Open Command Palette", "View")
                .with_description("Show the command palette search modal")
                .with_keybinding(KeyBinding::primary_shift("P")),
        );

        self.register_descriptor(
            CommandDescriptor::new("workspace.open", "Open Workspace...", "Workspace")
                .with_description("Select and open a local PacketSmith workspace directory")
                .with_keybinding(KeyBinding::primary("o")),
        );

        self.register_descriptor(
            CommandDescriptor::new("pane.split_vertical", "Split Editor Right", "View")
                .with_description("Split current active pane vertically")
                .with_keybinding(KeyBinding::primary("\\")),
        );
    }

    /// Registers a command descriptor without an action handler (useful for menu UI).
    pub fn register_descriptor(&mut self, descriptor: CommandDescriptor) {
        if let Some(ref binding) = descriptor.default_keybinding {
            self.keymap.insert(binding.clone(), descriptor.id.clone());
        }
        self.commands.insert(
            descriptor.id.clone(),
            RegisteredCommand {
                descriptor,
                action: None,
            },
        );
    }

    /// Registers a command with an executable action.
    pub fn register_action(
        &mut self,
        descriptor: CommandDescriptor,
        action: Arc<dyn CommandAction>,
    ) {
        if let Some(ref binding) = descriptor.default_keybinding {
            self.keymap.insert(binding.clone(), descriptor.id.clone());
        }
        self.commands.insert(
            descriptor.id.clone(),
            RegisteredCommand {
                descriptor,
                action: Some(action),
            },
        );
    }

    /// Retrieves a descriptor by CommandId.
    pub fn get_descriptor(&self, id: &CommandId) -> Option<&CommandDescriptor> {
        self.commands.get(id).map(|rc| &rc.descriptor)
    }

    /// Executes the specified command by ID.
    pub fn dispatch(&self, id: &CommandId) -> Result<(), CommandError> {
        let cmd = self
            .commands
            .get(id)
            .ok_or_else(|| CommandError::NotFound(id.to_string()))?;

        if !cmd.descriptor.enabled {
            return Err(CommandError::Disabled(id.to_string()));
        }

        if let Some(ref action) = cmd.action {
            action.execute()
        } else {
            Ok(())
        }
    }

    /// Finds a command assigned to a specific keybinding.
    pub fn command_for_keybinding(&self, binding: &KeyBinding) -> Option<&CommandId> {
        self.keymap.get(binding)
    }

    /// Searches commands by query for the Command Palette.
    pub fn search(&self, query: &str) -> Vec<&CommandDescriptor> {
        let q = query.to_lowercase();
        let mut matches: Vec<_> = self
            .commands
            .values()
            .map(|c| &c.descriptor)
            .filter(|d| {
                d.title.to_lowercase().contains(&q)
                    || d.category.to_lowercase().contains(&q)
                    || d.id.as_str().to_lowercase().contains(&q)
            })
            .collect();

        matches.sort_by(|a, b| a.title.cmp(&b.title));
        matches
    }

    /// Lists all registered command descriptors.
    pub fn list_all(&self) -> Vec<&CommandDescriptor> {
        let mut list: Vec<_> = self.commands.values().map(|c| &c.descriptor).collect();
        list.sort_by(|a, b| a.category.cmp(&b.category).then_with(|| a.title.cmp(&b.title)));
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_command_registry_initialization() {
        let registry = CommandRegistry::new();
        let all = registry.list_all();
        assert!(!all.is_empty());
        assert!(registry.get_descriptor(&CommandId::new("request.send")).is_some());
    }

    #[test]
    fn test_command_dispatch() {
        let mut registry = CommandRegistry::new();
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let desc = CommandDescriptor::new("test.action", "Test Action", "Test");
        registry.register_action(
            desc,
            Arc::new(move || {
                executed_clone.store(true, Ordering::SeqCst);
                Ok(())
            }),
        );

        registry.dispatch(&CommandId::new("test.action")).expect("dispatch failed");
        assert!(executed.load(Ordering::SeqCst));
    }

    #[test]
    fn test_search_commands() {
        let registry = CommandRegistry::new();
        let results = registry.search("request");
        assert!(results.iter().any(|d| d.id.as_str() == "request.send"));
        assert!(results.iter().any(|d| d.id.as_str() == "request.cancel"));
    }
}

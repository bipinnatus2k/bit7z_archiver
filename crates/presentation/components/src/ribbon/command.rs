use std::collections::HashMap;
use std::sync::Arc;

use gpui::SharedString;

/// Unique identifier for a command.
pub type CommandId = SharedString;

/// The context in which commands evaluate their enabled/visible state.
#[derive(Clone, Debug, Default)]
pub struct CommandContext {
    /// Current selection kind (e.g., "geometry_face", "mesh_element", "result_object").
    pub selection_kind: Option<SharedString>,
    /// Active workflow stage (e.g., "model", "mesh", "results").
    pub active_stage: Option<SharedString>,
    /// Whether a document/project is currently open.
    pub has_document: bool,
    /// Whether mesh data is loaded.
    pub has_mesh: bool,
    /// Whether result data is loaded.
    pub has_results: bool,
    /// The current active tool/mode (e.g., "section_plane", "probe").
    pub active_tool: Option<SharedString>,
    /// Custom key-value properties for extensibility.
    pub properties: HashMap<SharedString, SharedString>,
}

/// A predicate that determines command visibility/enable based on context.
pub type ContextPredicate = Arc<dyn Fn(&CommandContext) -> bool + Send + Sync>;

fn always_true(_: &CommandContext) -> bool {
    true
}

/// A command that can be executed from the ribbon, command palette, or keyboard shortcut.
#[derive(Clone)]
pub struct Command {
    pub id: CommandId,
    pub title: SharedString,
    pub icon: Option<SharedString>,
    pub shortcut: Option<SharedString>,
    pub category: Option<SharedString>,
    pub tooltip: Option<SharedString>,
    pub enabled: ContextPredicate,
    pub visible: ContextPredicate,
    pub handler: Arc<dyn Fn(&CommandContext) + Send + Sync>,
}

impl Command {
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: None,
            shortcut: None,
            category: None,
            tooltip: None,
            enabled: Arc::new(always_true),
            visible: Arc::new(always_true),
            handler: Arc::new(|_| {}),
        }
    }

    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn category(mut self, category: impl Into<SharedString>) -> Self {
        self.category = Some(category.into());
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn enabled_when(mut self, predicate: impl Fn(&CommandContext) -> bool + Send + Sync + 'static) -> Self {
        self.enabled = Arc::new(predicate);
        self
    }

    pub fn visible_when(mut self, predicate: impl Fn(&CommandContext) -> bool + Send + Sync + 'static) -> Self {
        self.visible = Arc::new(predicate);
        self
    }

    pub fn on_execute(mut self, handler: impl Fn(&CommandContext) + Send + Sync + 'static) -> Self {
        self.handler = Arc::new(handler);
        self
    }

    pub fn is_enabled(&self, ctx: &CommandContext) -> bool {
        (self.enabled)(ctx)
    }

    pub fn is_visible(&self, ctx: &CommandContext) -> bool {
        (self.visible)(ctx)
    }

    pub fn execute(&self, ctx: &CommandContext) {
        (self.handler)(ctx);
    }
}

/// Registry that holds all available commands.
#[derive(Clone, Default)]
pub struct CommandRegistry {
    commands: HashMap<CommandId, Command>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, command: Command) {
        self.commands.insert(command.id.clone(), command);
    }

    pub fn get(&self, id: &CommandId) -> Option<&Command> {
        self.commands.get(id)
    }

    pub fn all(&self) -> impl Iterator<Item = &Command> {
        self.commands.values()
    }

    pub fn search(&self, query: &str) -> Vec<&Command> {
        let query_lower = query.to_lowercase();
        self.commands
            .values()
            .filter(|cmd| {
                cmd.title.to_lowercase().contains(&query_lower)
                    || cmd.id.to_lowercase().contains(&query_lower)
                    || cmd
                        .category
                        .as_ref()
                        .map_or(false, |c| c.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    pub fn execute(&self, id: &CommandId, ctx: &CommandContext) {
        if let Some(cmd) = self.commands.get(id) {
            if cmd.is_enabled(ctx) {
                cmd.execute(ctx);
            }
        }
    }
}

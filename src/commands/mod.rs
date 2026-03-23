mod admin;
mod cv;
mod generation;
mod help;
mod jobs;
mod reminders;

pub use admin::{ClearAllCvsCommand, GetCvCommand, ListCvsCommand};
pub use cv::{DeleteCvCommand, ListMyCvsCommand, SendCvCommand};
pub use generation::{
    GenerateCoverLetterCommand, GenerateMarketAnalysisCommand, GenerateResumeCommand,
    SynthesizeOfferCommand,
};
pub use help::HelpCommand;
pub use jobs::{
    ApplyJobCommand, ApplicationHistoryCommand, MyStatsCommand, StatusCommand, UpdateStatusCommand,
    get_status_buttons, rebuild_tracking_embed_from_status,
};
pub use reminders::{
    SetReminderCommand, ListRemindersCommand, ClearReminderCommand,
    CreateReminderCommand, DeleteReminderCommand,
};

use async_trait::async_trait;
use serenity::all::{CommandInteraction, Context, CreateCommand};
use std::collections::HashMap;
use std::sync::Arc;

use crate::db::{Database, BaseCv};
use crate::services::ClaudeClient;
use crate::ClaudeClientKey;

/// Trait définissant une commande Discord slash
#[async_trait]
pub trait SlashCommand: Send + Sync {
    /// Nom de la commande
    fn name(&self) -> &'static str;

    /// Description de la commande
    fn description(&self) -> &'static str;

    /// Construit la définition de la commande pour Discord
    fn register(&self) -> CreateCommand;

    /// Exécute la commande
    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError>;
}

/// Erreur de commande
#[derive(Debug)]
#[allow(dead_code)]
pub enum CommandError {
    /// Erreur lors de l'envoi de la réponse
    ResponseFailed(String),
    /// Paramètre manquant
    MissingParameter(String),
    /// Permission refusée
    PermissionDenied,
    /// Ressource non trouvée
    NotFound(String),
    /// Non autorisé
    Unauthorized(String),
    /// Input invalide
    InvalidInput(String),
    /// Erreur interne
    Internal(String),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::ResponseFailed(msg) => write!(f, "Failed to send response: {}", msg),
            CommandError::MissingParameter(param) => write!(f, "Missing parameter: {}", param),
            CommandError::PermissionDenied => write!(f, "Permission denied"),
            CommandError::NotFound(msg) => write!(f, "Not found: {}", msg),
            CommandError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            CommandError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            CommandError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for CommandError {}

/// Registre centralisé de toutes les commandes
pub struct CommandRegistry {
    commands: HashMap<&'static str, Box<dyn SlashCommand>>,
    order: Vec<&'static str>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self { commands: HashMap::new(), order: Vec::new() }
    }

    /// Enregistre une nouvelle commande
    pub fn register<C: SlashCommand + 'static>(&mut self, command: C) -> &mut Self {
        let name = command.name();
        self.order.push(name);
        self.commands.insert(name, Box::new(command));
        self
    }

    /// Retourne toutes les définitions de commandes pour l'enregistrement Discord
    pub fn build_commands(&self) -> Vec<CreateCommand> {
        self.order.iter()
            .filter_map(|name| self.commands.get(name))
            .map(|cmd| cmd.register())
            .collect()
    }

    /// Trouve et exécute une commande par son nom (O(1) lookup)
    pub async fn dispatch(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        let command_name = interaction.data.name.as_str();
        if let Some(cmd) = self.commands.get(command_name) {
            cmd.execute(ctx, interaction).await
        } else {
            Err(CommandError::Internal(format!("Unknown command: {}", command_name)))
        }
    }

    /// Retourne les informations d'aide pour toutes les commandes
    pub fn help_info(&self) -> Vec<(&'static str, &'static str)> {
        self.order.iter()
            .filter_map(|name| self.commands.get(name))
            .map(|cmd| (cmd.name(), cmd.description()))
            .collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Dependency-Injection helpers (évitent le boilerplate dans chaque commande)
// ============================================================================

/// Récupère le ClaudeClient depuis le TypeMap de Serenity.
pub async fn get_claude_client(ctx: &Context) -> Result<Arc<ClaudeClient>, CommandError> {
    ctx.data
        .read()
        .await
        .get::<ClaudeClientKey>()
        .cloned()
        .ok_or_else(|| CommandError::Internal("Claude client not found".to_string()))
}

/// Récupère la Database depuis le TypeMap de Serenity.
pub async fn get_database(ctx: &Context) -> Result<Database, CommandError> {
    ctx.data
        .read()
        .await
        .get::<Database>()
        .cloned()
        .ok_or_else(|| CommandError::Internal("Database not found".to_string()))
}

/// Retourne le texte du CV : priorité à extracted_text, sinon lecture du fichier.
pub async fn get_cv_text(cv: &BaseCv) -> String {
    if let Some(ref text) = cv.extracted_text {
        if !text.is_empty() {
            return text.clone();
        }
    }
    tokio::fs::read_to_string(&cv.file_path)
        .await
        .unwrap_or_else(|_| format!("CV: {} (texte non disponible)", cv.original_name))
}
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
    ApplyJobCommand, MyStatsCommand, StatusCommand, UpdateStatusCommand,
    get_status_buttons, rebuild_tracking_embed_from_status,
};
pub use reminders::{
    SetReminderCommand, ListRemindersCommand, ClearReminderCommand,
    CreateReminderCommand, DeleteReminderCommand,
};

use async_trait::async_trait;
use serenity::all::{CommandInteraction, Context, CreateCommand};

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
    commands: Vec<Box<dyn SlashCommand>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self { commands: Vec::new() }
    }

    /// Enregistre une nouvelle commande
    pub fn register<C: SlashCommand + 'static>(&mut self, command: C) -> &mut Self {
        self.commands.push(Box::new(command));
        self
    }

    /// Retourne toutes les définitions de commandes pour l'enregistrement Discord
    pub fn build_commands(&self) -> Vec<CreateCommand> {
        self.commands.iter().map(|cmd| cmd.register()).collect()
    }

    /// Trouve et exécute une commande par son nom
    pub async fn dispatch(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        let command_name = interaction.data.name.as_str();
        
        if let Some(cmd) = self.commands.iter().find(|c| c.name() == command_name) {
            cmd.execute(ctx, interaction).await
        } else {
            Err(CommandError::Internal(format!("Unknown command: {}", command_name)))
        }
    }

    /// Retourne les informations d'aide pour toutes les commandes
    pub fn help_info(&self) -> Vec<(&'static str, &'static str)> {
        self.commands
            .iter()
            .map(|cmd| (cmd.name(), cmd.description()))
            .collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}
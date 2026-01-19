use async_trait::async_trait;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse,
};

use super::{CommandError, SlashCommand};

// ============================================================================
// SendCV Command
// ============================================================================

pub struct SendCvCommand;

impl SendCvCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SendCvCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for SendCvCommand {
    fn name(&self) -> &'static str {
        "sendcv"
    }

    fn description(&self) -> &'static str {
        "Upload your CV to the bot"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description(self.description())
            .add_option(
                CreateCommandOption::new(CommandOptionType::Attachment, "cv", "Your CV file (PDF preferred)")
                    .required(true),
            )
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        // Defer immédiatement pour éviter le timeout de 3s
        interaction
            .defer(&ctx.http)
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        let user_id = interaction.user.id;
        
        // Récupérer l'attachment
        let attachment_id = interaction
            .data
            .options
            .iter()
            .find(|opt| opt.name == "cv")
            .and_then(|opt| opt.value.as_attachment_id())
            .ok_or_else(|| CommandError::MissingParameter("cv".to_string()))?;

        // Récupérer les infos de l'attachment depuis resolved
        let attachment = interaction
            .data
            .resolved
            .attachments
            .get(&attachment_id)
            .ok_or_else(|| CommandError::Internal("Attachment not found in resolved data".to_string()))?;

        // TODO: Télécharger et stocker le CV
        let response = format!(
            "✅ CV reçu pour <@{}>!\n📄 Fichier: `{}`\n📦 Taille: {} bytes\n\n_Stockage à implémenter_",
            user_id,
            attachment.filename,
            attachment.size
        );

        // Répondre avec edit (car on a defer)
        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().content(response))
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        Ok(())
    }
}

// ============================================================================
// DeleteCV Command
// ============================================================================

pub struct DeleteCvCommand;

impl DeleteCvCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DeleteCvCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for DeleteCvCommand {
    fn name(&self) -> &'static str {
        "deletecv"
    }

    fn description(&self) -> &'static str {
        "Delete your CV from the bot"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name()).description(self.description())
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        let user_id = interaction.user.id;
        
        // TODO: Implémenter la suppression du CV
        let response = format!("🗑️ CV deletion for <@{}> — coming soon!", user_id);
        
        send_response(ctx, interaction, &response).await
    }
}

// ============================================================================
// ListMyCvs Command
// ============================================================================

pub struct ListMyCvsCommand;

impl ListMyCvsCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ListMyCvsCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for ListMyCvsCommand {
    fn name(&self) -> &'static str {
        "listmycvs"
    }

    fn description(&self) -> &'static str {
        "List your stored CVs"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name()).description(self.description())
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        let user_id = interaction.user.id;
        
        // TODO: Récupérer la liste des CVs de l'utilisateur
        let response = format!("📋 Your CVs, <@{}>:\n• No CVs stored yet.", user_id);
        
        send_response(ctx, interaction, &response).await
    }
}

// ============================================================================
// Helper
// ============================================================================

async fn send_response(
    ctx: &Context,
    interaction: &CommandInteraction,
    content: &str,
) -> Result<(), CommandError> {
    let msg = CreateInteractionResponseMessage::new().content(content);
    
    interaction
        .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
        .await
        .map_err(|e| CommandError::ResponseFailed(e.to_string()))
}
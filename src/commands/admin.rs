use async_trait::async_trait;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage, Permissions,
};

use super::{CommandError, SlashCommand};

// ============================================================================
// ListCvs Command (Admin)
// ============================================================================

pub struct ListCvsCommand;

impl ListCvsCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ListCvsCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for ListCvsCommand {
    fn name(&self) -> &'static str {
        "listcvs"
    }

    fn description(&self) -> &'static str {
        "List all stored CVs (admin only)"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description(self.description())
            .default_member_permissions(Permissions::ADMINISTRATOR)
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        // Vérification des permissions côté serveur aussi
        if !has_admin_permission(interaction) {
            return send_response(ctx, interaction, "❌ You need administrator permissions.").await;
        }

        // TODO: Récupérer tous les CVs
        let response = "📋 **All stored CVs:**\n• No CVs in database yet.";
        
        send_response(ctx, interaction, response).await
    }
}

// ============================================================================
// GetCv Command (Admin)
// ============================================================================

pub struct GetCvCommand;

impl GetCvCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GetCvCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for GetCvCommand {
    fn name(&self) -> &'static str {
        "getcv"
    }

    fn description(&self) -> &'static str {
        "Retrieve a specific CV by user (admin only)"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description(self.description())
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "user", "User to get CV from")
                    .required(true),
            )
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        if !has_admin_permission(interaction) {
            return send_response(ctx, interaction, "❌ You need administrator permissions.").await;
        }

        let _target_user = interaction
            .data
            .options
            .iter()
            .find(|opt| opt.name == "user")
            .ok_or_else(|| CommandError::MissingParameter("user".to_string()))?;

        // TODO: Récupérer le CV de l'utilisateur ciblé
        let response = "📄 CV retrieval — coming soon!";
        
        send_response(ctx, interaction, response).await
    }
}

// ============================================================================
// ClearAllCvs Command (Admin)
// ============================================================================

pub struct ClearAllCvsCommand;

impl ClearAllCvsCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClearAllCvsCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for ClearAllCvsCommand {
    fn name(&self) -> &'static str {
        "clearallcvs"
    }

    fn description(&self) -> &'static str {
        "Delete all stored CVs (admin only)"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description(self.description())
            .default_member_permissions(Permissions::ADMINISTRATOR)
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        if !has_admin_permission(interaction) {
            return send_response(ctx, interaction, "❌ You need administrator permissions.").await;
        }

        // TODO: Implémenter la suppression de tous les CVs (avec confirmation!)
        let response = "⚠️ This will delete ALL CVs. Confirmation system coming soon!";
        
        send_response(ctx, interaction, response).await
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn has_admin_permission(interaction: &CommandInteraction) -> bool {
    interaction
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| p.administrator())
        .unwrap_or(false)
}

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

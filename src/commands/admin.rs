use async_trait::async_trait;
use serenity::all::{
    ButtonStyle, CommandInteraction, CommandOptionType, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
    Permissions,
};

use super::{CommandError, SlashCommand, get_database};

fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut boundary = max_bytes;
    while !s.is_char_boundary(boundary) {
        boundary -= 1;
    }
    &s[..boundary]
}

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
        if !has_admin_permission(interaction) {
            return send_response(ctx, interaction, "❌ You need administrator permissions.").await;
        }

        let db = get_database(ctx).await?;
        let cvs = db.list_all_cvs().await
            .map_err(|e| CommandError::Internal(format!("DB error: {}", e)))?;

        if cvs.is_empty() {
            return send_response(ctx, interaction, "📋 **All stored CVs:**\n• No CVs in database.").await;
        }

        let mut lines = vec!["📋 **All stored CVs:**".to_string()];
        for (user_id, username, cv) in &cvs {
            lines.push(format!(
                "• **{}** (ID: {}) — `{}` — {} bytes — {}",
                username, user_id, cv.original_name, cv.file_size, cv.created_at
            ));
        }
        let response = lines.join("\n");
        send_response(ctx, interaction, safe_truncate(&response, 1900)).await
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

        let target_user_id = interaction
            .data
            .options
            .iter()
            .find(|opt| opt.name == "user")
            .and_then(|opt| opt.value.as_user_id())
            .ok_or_else(|| CommandError::MissingParameter("user".to_string()))?;

        let db = get_database(ctx).await?;
        let cv = db.get_active_cv(target_user_id.get() as i64).await
            .map_err(|e| CommandError::Internal(format!("DB error: {}", e)))?;

        match cv {
            None => send_response(ctx, interaction, &format!("📄 No active CV for <@{}>.", target_user_id)).await,
            Some(cv) => {
                let preview = cv.extracted_text.as_deref()
                    .filter(|t| !t.is_empty())
                    .map(|t| safe_truncate(t, 500))
                    .unwrap_or("(no extracted text)");
                let response = format!(
                    "📄 **CV for <@{}>**\n\
                     • File: `{}`\n\
                     • Size: {} bytes\n\
                     • Uploaded: {}\n\
                     • Preview:\n```\n{}\n```",
                    target_user_id, cv.original_name, cv.file_size, cv.created_at, preview
                );
                send_response(ctx, interaction, safe_truncate(&response, 1900)).await
            }
        }
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

        let confirm_btn = CreateButton::new("clearallcvs_confirm")
            .label("Confirmer suppression")
            .style(ButtonStyle::Danger);
        let cancel_btn = CreateButton::new("clearallcvs_cancel")
            .label("Annuler")
            .style(ButtonStyle::Secondary);
        let row = CreateActionRow::Buttons(vec![confirm_btn, cancel_btn]);

        let msg = CreateInteractionResponseMessage::new()
            .content("⚠️ **Êtes-vous sûr de vouloir supprimer TOUS les CVs ?** Cette action est irréversible.")
            .components(vec![row]);
        interaction
            .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))
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

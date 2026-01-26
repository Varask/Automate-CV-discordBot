use async_trait::async_trait;
use serenity::all::{
    Colour, CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, EditInteractionResponse,
};
use tracing::info;
use chrono::{NaiveDateTime, Utc, Duration};

use super::{CommandError, SlashCommand};
use crate::db::Database;

const COLOR_REMINDER: Colour = Colour::from_rgb(241, 196, 15);

// ============================================================================
// SetReminder Command - Set a reminder for an application
// ============================================================================

pub struct SetReminderCommand;

impl SetReminderCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SetReminderCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for SetReminderCommand {
    fn name(&self) -> &'static str {
        "setreminder"
    }

    fn description(&self) -> &'static str {
        "Set a follow-up reminder for a job application"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description(self.description())
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "application_id",
                    "Application ID to set reminder for",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "days",
                    "Number of days from now for the reminder (default: 7)",
                )
                .required(false)
                .min_int_value(1)
                .max_int_value(90),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "date",
                    "Specific date for reminder (YYYY-MM-DD format)",
                )
                .required(false),
            )
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        interaction.defer(&ctx.http).await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        let user_id = interaction.user.id.get() as i64;

        // Get application_id
        let application_id = interaction
            .data
            .options
            .iter()
            .find(|opt| opt.name == "application_id")
            .and_then(|opt| opt.value.as_i64())
            .ok_or_else(|| CommandError::MissingParameter("application_id".to_string()))?;

        // Get days or date
        let days = interaction
            .data
            .options
            .iter()
            .find(|opt| opt.name == "days")
            .and_then(|opt| opt.value.as_i64());

        let date_str = interaction
            .data
            .options
            .iter()
            .find(|opt| opt.name == "date")
            .and_then(|opt| opt.value.as_str())
            .map(|s| s.to_string());

        // Get database
        let db = {
            let data = ctx.data.read().await;
            data.get::<Database>()
                .ok_or_else(|| CommandError::Internal("Database not found".to_string()))?
                .clone()
        };

        // Verify application exists and belongs to user
        let app = db.get_application(application_id)
            .map_err(|e| CommandError::Internal(format!("Database error: {}", e)))?
            .ok_or_else(|| CommandError::NotFound("Application not found".to_string()))?;

        if app.user_id != user_id {
            return Err(CommandError::Unauthorized("This application does not belong to you".to_string()));
        }

        // Calculate reminder date
        let reminder_datetime = if let Some(date) = date_str {
            NaiveDateTime::parse_from_str(&format!("{} 09:00:00", date), "%Y-%m-%d %H:%M:%S")
                .map_err(|_| CommandError::InvalidInput("Invalid date format. Use YYYY-MM-DD".to_string()))?
        } else {
            let days_offset = days.unwrap_or(7);
            (Utc::now() + Duration::days(days_offset)).naive_utc()
        };

        let reminder_date_str = reminder_datetime.format("%Y-%m-%d %H:%M:%S").to_string();

        // Set reminder
        db.set_application_reminder(application_id, &reminder_date_str)
            .map_err(|e| CommandError::Internal(format!("Failed to set reminder: {}", e)))?;

        info!("Set reminder for application {} on {}", application_id, reminder_date_str);

        let embed = CreateEmbed::new()
            .title("Rappel programme")
            .colour(COLOR_REMINDER)
            .field("Candidature", format!("#{} - {} chez {}",
                application_id,
                app.job_title.as_deref().unwrap_or("N/A"),
                app.company.as_deref().unwrap_or("N/A")
            ), false)
            .field("Date de rappel", reminder_datetime.format("%d/%m/%Y a %H:%M").to_string(), true)
            .field("Statut actuel", &app.status, true)
            .footer(serenity::all::CreateEmbedFooter::new(
                "Vous recevrez une notification automatique a cette date"
            ));

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        Ok(())
    }
}

// ============================================================================
// ListReminders Command - List all pending reminders
// ============================================================================

pub struct ListRemindersCommand;

impl ListRemindersCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ListRemindersCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for ListRemindersCommand {
    fn name(&self) -> &'static str {
        "listreminders"
    }

    fn description(&self) -> &'static str {
        "List all your pending reminders"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name()).description(self.description())
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        interaction.defer(&ctx.http).await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        let user_id = interaction.user.id.get() as i64;

        let db = {
            let data = ctx.data.read().await;
            data.get::<Database>()
                .ok_or_else(|| CommandError::Internal("Database not found".to_string()))?
                .clone()
        };

        // Get application reminders
        let app_reminders = db.list_user_application_reminders(user_id)
            .map_err(|e| CommandError::Internal(format!("Database error: {}", e)))?;

        // Get standalone reminders
        let standalone_reminders = db.list_user_reminders(user_id)
            .map_err(|e| CommandError::Internal(format!("Database error: {}", e)))?;

        if app_reminders.is_empty() && standalone_reminders.is_empty() {
            let embed = CreateEmbed::new()
                .title("Mes Rappels")
                .colour(COLOR_REMINDER)
                .description("Aucun rappel programme.\n\nUtilisez `/setreminder` pour programmer un rappel de suivi.");

            interaction
                .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
                .await
                .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

            return Ok(());
        }

        let mut description = String::new();

        // Application reminders
        if !app_reminders.is_empty() {
            description.push_str("**Rappels de candidatures:**\n");
            for app in app_reminders.iter().take(10) {
                let date = app.reminder_date.as_deref().unwrap_or("N/A");
                let formatted_date = if let Ok(dt) = NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S") {
                    dt.format("%d/%m/%Y").to_string()
                } else {
                    date.to_string()
                };
                description.push_str(&format!(
                    "- **#{}** {} @ {} - `{}`\n",
                    app.id,
                    app.job_title.as_deref().unwrap_or("N/A"),
                    app.company.as_deref().unwrap_or("N/A"),
                    formatted_date
                ));
            }
            description.push('\n');
        }

        // Standalone reminders
        if !standalone_reminders.is_empty() {
            description.push_str("**Autres rappels:**\n");
            for reminder in standalone_reminders.iter().take(10) {
                let formatted_date = if let Ok(dt) = NaiveDateTime::parse_from_str(&reminder.reminder_date, "%Y-%m-%d %H:%M:%S") {
                    dt.format("%d/%m/%Y").to_string()
                } else {
                    reminder.reminder_date.clone()
                };
                description.push_str(&format!(
                    "- **#{}** {} - `{}`\n",
                    reminder.id,
                    &reminder.message[..reminder.message.len().min(50)],
                    formatted_date
                ));
            }
        }

        let total = app_reminders.len() + standalone_reminders.len();
        let embed = CreateEmbed::new()
            .title(format!("Mes Rappels ({})", total))
            .colour(COLOR_REMINDER)
            .description(description)
            .footer(serenity::all::CreateEmbedFooter::new(
                "Utilisez /clearreminder pour supprimer un rappel"
            ));

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        Ok(())
    }
}

// ============================================================================
// ClearReminder Command - Remove a reminder
// ============================================================================

pub struct ClearReminderCommand;

impl ClearReminderCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClearReminderCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for ClearReminderCommand {
    fn name(&self) -> &'static str {
        "clearreminder"
    }

    fn description(&self) -> &'static str {
        "Clear a reminder from an application"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description(self.description())
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "application_id",
                    "Application ID to clear reminder from",
                )
                .required(true),
            )
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        interaction.defer(&ctx.http).await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        let user_id = interaction.user.id.get() as i64;

        let application_id = interaction
            .data
            .options
            .iter()
            .find(|opt| opt.name == "application_id")
            .and_then(|opt| opt.value.as_i64())
            .ok_or_else(|| CommandError::MissingParameter("application_id".to_string()))?;

        let db = {
            let data = ctx.data.read().await;
            data.get::<Database>()
                .ok_or_else(|| CommandError::Internal("Database not found".to_string()))?
                .clone()
        };

        // Verify application exists and belongs to user
        let app = db.get_application(application_id)
            .map_err(|e| CommandError::Internal(format!("Database error: {}", e)))?
            .ok_or_else(|| CommandError::NotFound("Application not found".to_string()))?;

        if app.user_id != user_id {
            return Err(CommandError::Unauthorized("This application does not belong to you".to_string()));
        }

        // Clear reminder
        db.clear_application_reminder(application_id)
            .map_err(|e| CommandError::Internal(format!("Failed to clear reminder: {}", e)))?;

        info!("Cleared reminder for application {}", application_id);

        let embed = CreateEmbed::new()
            .title("Rappel supprime")
            .colour(Colour::from_rgb(46, 204, 113))
            .description(format!(
                "Le rappel pour la candidature **#{}** ({} chez {}) a ete supprime.",
                application_id,
                app.job_title.as_deref().unwrap_or("N/A"),
                app.company.as_deref().unwrap_or("N/A")
            ));

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        Ok(())
    }
}

// ============================================================================
// CreateReminder Command - Create a standalone reminder
// ============================================================================

pub struct CreateReminderCommand;

impl CreateReminderCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CreateReminderCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for CreateReminderCommand {
    fn name(&self) -> &'static str {
        "createreminder"
    }

    fn description(&self) -> &'static str {
        "Create a custom reminder (not linked to an application)"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description(self.description())
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "message",
                    "Reminder message",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "days",
                    "Number of days from now (default: 1)",
                )
                .required(false)
                .min_int_value(1)
                .max_int_value(365),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "date",
                    "Specific date (YYYY-MM-DD format)",
                )
                .required(false),
            )
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        interaction.defer(&ctx.http).await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        let user_id = interaction.user.id.get() as i64;
        let channel_id = interaction.channel_id.get() as i64;

        let message = interaction
            .data
            .options
            .iter()
            .find(|opt| opt.name == "message")
            .and_then(|opt| opt.value.as_str())
            .ok_or_else(|| CommandError::MissingParameter("message".to_string()))?
            .to_string();

        let days = interaction
            .data
            .options
            .iter()
            .find(|opt| opt.name == "days")
            .and_then(|opt| opt.value.as_i64());

        let date_str = interaction
            .data
            .options
            .iter()
            .find(|opt| opt.name == "date")
            .and_then(|opt| opt.value.as_str())
            .map(|s| s.to_string());

        let db = {
            let data = ctx.data.read().await;
            data.get::<Database>()
                .ok_or_else(|| CommandError::Internal("Database not found".to_string()))?
                .clone()
        };

        // Calculate reminder date
        let reminder_datetime = if let Some(date) = date_str {
            NaiveDateTime::parse_from_str(&format!("{} 09:00:00", date), "%Y-%m-%d %H:%M:%S")
                .map_err(|_| CommandError::InvalidInput("Invalid date format. Use YYYY-MM-DD".to_string()))?
        } else {
            let days_offset = days.unwrap_or(1);
            (Utc::now() + Duration::days(days_offset)).naive_utc()
        };

        let reminder_date_str = reminder_datetime.format("%Y-%m-%d %H:%M:%S").to_string();

        // Create reminder
        let reminder_id = db.create_reminder(user_id, None, channel_id, &reminder_date_str, &message)
            .map_err(|e| CommandError::Internal(format!("Failed to create reminder: {}", e)))?;

        info!("Created standalone reminder {} for user {}", reminder_id, user_id);

        let embed = CreateEmbed::new()
            .title("Rappel cree")
            .colour(COLOR_REMINDER)
            .field("ID", format!("#{}", reminder_id), true)
            .field("Date", reminder_datetime.format("%d/%m/%Y a %H:%M").to_string(), true)
            .field("Message", &message, false)
            .footer(serenity::all::CreateEmbedFooter::new(
                "Vous serez notifie dans ce canal a la date prevue"
            ));

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        Ok(())
    }
}

// ============================================================================
// DeleteReminder Command - Delete a standalone reminder
// ============================================================================

pub struct DeleteReminderCommand;

impl DeleteReminderCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DeleteReminderCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for DeleteReminderCommand {
    fn name(&self) -> &'static str {
        "deletereminder"
    }

    fn description(&self) -> &'static str {
        "Delete a custom reminder by its ID"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description(self.description())
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "reminder_id",
                    "Reminder ID to delete",
                )
                .required(true),
            )
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        interaction.defer_ephemeral(&ctx.http).await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        let user_id = interaction.user.id.get() as i64;

        let reminder_id = interaction
            .data
            .options
            .iter()
            .find(|opt| opt.name == "reminder_id")
            .and_then(|opt| opt.value.as_i64())
            .ok_or_else(|| CommandError::MissingParameter("reminder_id".to_string()))?;

        let db = {
            let data = ctx.data.read().await;
            data.get::<Database>()
                .ok_or_else(|| CommandError::Internal("Database not found".to_string()))?
                .clone()
        };

        let deleted = db.delete_reminder(reminder_id, user_id)
            .map_err(|e| CommandError::Internal(format!("Failed to delete reminder: {}", e)))?;

        if !deleted {
            return Err(CommandError::NotFound("Reminder not found or does not belong to you".to_string()));
        }

        info!("Deleted reminder {} for user {}", reminder_id, user_id);

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new()
                .content(format!("Rappel #{} supprime avec succes.", reminder_id)))
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        Ok(())
    }
}

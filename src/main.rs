mod commands;
mod db;
mod services;

use commands::{
    ApplyJobCommand, ClearAllCvsCommand, CommandRegistry, DeleteCvCommand,
    GenerateCoverLetterCommand, GenerateMarketAnalysisCommand, GenerateResumeCommand,
    GetCvCommand, HelpCommand, ListCvsCommand, ListMyCvsCommand, MyStatsCommand,
    SendCvCommand, StatusCommand, SynthesizeOfferCommand, UpdateStatusCommand,
    get_status_buttons, rebuild_tracking_embed_from_status,
    // Reminder commands
    SetReminderCommand, ListRemindersCommand, ClearReminderCommand,
    CreateReminderCommand, DeleteReminderCommand,
};
use db::Database;
use services::ClaudeClient;
use serenity::all::{ChannelId, GatewayIntents, GuildId, Interaction, UserId};
use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

/// Clé pour stocker le registre de commandes dans le TypeMap de Serenity
struct CommandRegistryKey;

impl TypeMapKey for CommandRegistryKey {
    type Value = Arc<CommandRegistry>;
}

/// Clé pour stocker le client Claude dans le TypeMap de Serenity
pub struct ClaudeClientKey;

impl TypeMapKey for ClaudeClientKey {
    type Value = Arc<ClaudeClient>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("✅ {} is now online!", ready.user.name);

        // Récupérer le registre depuis le TypeMap
        let registry = {
            let data = ctx.data.read().await;
            data.get::<CommandRegistryKey>()
                .expect("CommandRegistry not found in TypeMap")
                .clone()
        };

        // Construire les commandes
        let commands = registry.build_commands();

        // Enregistrer les commandes (guild pour dev, global pour prod)
        let guild_id = env::var("GUILD_ID")
            .ok()
            .and_then(|id| id.parse::<u64>().ok())
            .map(GuildId::new);

        if let Some(guild) = guild_id {
            match guild.set_commands(&ctx.http, commands).await {
                Ok(_) => info!("🔧 Registered {} guild commands", registry.build_commands().len()),
                Err(e) => error!("Failed to register guild commands: {}", e),
            }
        } else {
            for cmd in commands {
                if let Err(e) =
                    serenity::model::application::Command::create_global_command(&ctx.http, cmd).await
                {
                    error!("Failed to register global command: {}", e);
                }
            }
            info!("🌍 Registered global commands");
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(cmd) => {
                // Récupérer le registre
                let registry = {
                    let data = ctx.data.read().await;
                    data.get::<CommandRegistryKey>()
                        .expect("CommandRegistry not found")
                        .clone()
                };

                // Dispatcher la commande
                if let Err(e) = registry.dispatch(&ctx, &cmd).await {
                    error!("Command error: {}", e);

                    // Tenter d'envoyer un message d'erreur à l'utilisateur
                    let _ = cmd
                        .create_response(
                            &ctx.http,
                            serenity::all::CreateInteractionResponse::Message(
                                serenity::all::CreateInteractionResponseMessage::new()
                                    .content(format!("❌ Error: {}", e))
                                    .ephemeral(true),
                            ),
                        )
                        .await;
                }
            }
            Interaction::Component(component) => {
                // Gérer les clics sur les boutons de statut
                if let Err(e) = handle_component_interaction(&ctx, &component).await {
                    error!("Component interaction error: {}", e);
                    let _ = component
                        .create_response(
                            &ctx.http,
                            serenity::all::CreateInteractionResponse::Message(
                                serenity::all::CreateInteractionResponseMessage::new()
                                    .content(format!("❌ Erreur: {}", e))
                                    .ephemeral(true),
                            ),
                        )
                        .await;
                }
            }
            _ => {}
        }
    }
}

/// Gère les interactions avec les composants (boutons)
async fn handle_component_interaction(
    ctx: &Context,
    component: &serenity::all::ComponentInteraction,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let custom_id = &component.data.custom_id;

    // Format: status_{application_id}_{new_status}
    if custom_id.starts_with("status_") {
        let parts: Vec<&str> = custom_id.split('_').collect();
        if parts.len() >= 3 {
            let application_id: i64 = parts[1].parse()?;
            let new_status = parts[2];
            let user_id = component.user.id.get() as i64;

            info!(
                "Status update: user {} changing application {} to {}",
                user_id, application_id, new_status
            );

            // Récupérer la DB
            let db = {
                let data = ctx.data.read().await;
                data.get::<Database>()
                    .ok_or("Database not found")?
                    .clone()
            };

            // Mettre à jour le statut en DB
            let updated = db.update_application_status(application_id, user_id, new_status, None)?;

            if !updated {
                return Err("Cette candidature ne vous appartient pas ou n'existe pas.".into());
            }

            // Récupérer l'application mise à jour pour reconstruire l'embed
            let app = db
                .get_application(application_id)?
                .ok_or("Application not found after update")?;

            // Reconstruire l'embed avec le nouveau statut
            let thread_id = app.thread_id.map(|t| t as u64);
            let embed = rebuild_tracking_embed_from_status(
                app.company.as_deref().unwrap_or("N/A"),
                app.job_title.as_deref().unwrap_or("N/A"),
                app.location.as_deref().unwrap_or("N/A"),
                app.match_score.unwrap_or(0) as u32,
                app.generated_cv_path.is_some(),
                thread_id,
                application_id,
                new_status,
            );

            // Reconstruire les boutons
            let buttons = get_status_buttons(application_id, new_status);

            // Mettre à jour le message avec le nouvel embed et les nouveaux boutons
            component
                .create_response(
                    &ctx.http,
                    serenity::all::CreateInteractionResponse::UpdateMessage(
                        serenity::all::CreateInteractionResponseMessage::new()
                            .embed(embed)
                            .components(buttons),
                    ),
                )
                .await?;

            info!(
                "Successfully updated application {} to status {}",
                application_id, new_status
            );
        }
    }

    Ok(())
}

/// Tache de fond pour verifier et envoyer les rappels automatiques
async fn reminder_check_task(http: Arc<serenity::http::Http>, db: Database) {
    info!("Starting reminder check background task");

    loop {
        // Check every 5 minutes
        tokio::time::sleep(Duration::from_secs(300)).await;

        // Check application reminders
        match db.get_pending_application_reminders() {
            Ok(apps) => {
                for app in apps {
                    info!("Sending reminder for application {} to user {}", app.id, app.user_id);

                    // Try to DM the user
                    let user_id = UserId::new(app.user_id as u64);
                    match user_id.create_dm_channel(&http).await {
                        Ok(dm_channel) => {
                            let message = format!(
                                "**Rappel de suivi de candidature**\n\n\
                                Candidature **#{}** - {} chez {}\n\
                                Statut actuel: `{}`\n\n\
                                N'oubliez pas de faire le suivi de cette candidature!\n\
                                Utilisez `/status` pour voir vos candidatures.",
                                app.id,
                                app.job_title.as_deref().unwrap_or("N/A"),
                                app.company.as_deref().unwrap_or("N/A"),
                                app.status
                            );

                            if let Err(e) = dm_channel.say(&http, &message).await {
                                error!("Failed to send reminder DM: {}", e);
                            } else {
                                // Mark as sent
                                if let Err(e) = db.mark_application_reminder_sent(app.id) {
                                    error!("Failed to mark reminder as sent: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to create DM channel for user {}: {}", app.user_id, e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to get pending application reminders: {}", e);
            }
        }

        // Check standalone reminders
        match db.get_pending_reminders() {
            Ok(reminders) => {
                for reminder in reminders {
                    info!("Sending standalone reminder {} to user {}", reminder.id, reminder.user_id);

                    // Send to the specified channel
                    let channel_id = ChannelId::new(reminder.channel_id as u64);
                    let user_mention = format!("<@{}>", reminder.user_id);

                    let message = format!(
                        "{} **Rappel**\n\n{}",
                        user_mention,
                        reminder.message
                    );

                    if let Err(e) = channel_id.say(&http, &message).await {
                        error!("Failed to send reminder to channel: {}", e);
                        // Try DM as fallback
                        let user_id = UserId::new(reminder.user_id as u64);
                        if let Ok(dm_channel) = user_id.create_dm_channel(&http).await {
                            let _ = dm_channel.say(&http, &format!("**Rappel**\n\n{}", reminder.message)).await;
                        }
                    }

                    // Mark as sent
                    if let Err(e) = db.mark_reminder_sent(reminder.id) {
                        error!("Failed to mark standalone reminder as sent: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to get pending reminders: {}", e);
            }
        }
    }
}

/// Initialise le registre avec toutes les commandes
fn build_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    // === CORE USER COMMANDS ===
    // CV Management
    registry
        .register(SendCvCommand::new())
        .register(DeleteCvCommand::new())
        .register(ListMyCvsCommand::new());

    // Job Application Pipeline (main workflow)
    registry
        .register(ApplyJobCommand::new())
        .register(StatusCommand::new())
        .register(UpdateStatusCommand::new())
        .register(MyStatsCommand::new());

    // === ADMIN COMMANDS ===
    registry
        .register(ListCvsCommand::new())
        .register(GetCvCommand::new())
        .register(ClearAllCvsCommand::new());

    // === LEGACY/STANDALONE AI COMMANDS ===
    // (kept for direct access, but /applyjob combines them)
    registry
        .register(SynthesizeOfferCommand::new())
        .register(GenerateResumeCommand::new())
        .register(GenerateCoverLetterCommand::new())
        .register(GenerateMarketAnalysisCommand::new());

    // === REMINDER COMMANDS ===
    registry
        .register(SetReminderCommand::new())
        .register(ListRemindersCommand::new())
        .register(ClearReminderCommand::new())
        .register(CreateReminderCommand::new())
        .register(DeleteReminderCommand::new());

    // Help command (created last to include all commands)
    let help_info = registry.help_info();
    registry.register(HelpCommand::new(help_info));

    registry
}

#[tokio::main]
async fn main() {
    // Initialiser le logging
    tracing_subscriber::fmt::init();

    // Charger les variables d'environnement
    dotenv::dotenv().ok();

    // Initialiser la base de données
    let database = Database::new().expect("Failed to initialize database");

    // Initialiser le client Claude (HTTP)
    let claude_client = Arc::new(ClaudeClient::from_env());

    // Vérifier la connexion au serveur Claude
    match claude_client.health_check().await {
        Ok(true) => info!("🤖 Connected to Claude HTTP server"),
        Ok(false) => warn!("⚠️ Claude server responded but not healthy"),
        Err(e) => warn!("⚠️ Claude connection failed (will retry on demand): {}", e),
    }

    let token = env::var("DISCORD_BOT_TOKEN").expect("Expected DISCORD_BOT_TOKEN in .env");

    // Construire le registre de commandes
    let registry = Arc::new(build_registry());

    // Créer le client Discord
    let mut client = Client::builder(&token, GatewayIntents::empty())
        .event_handler(Handler)
        .await
        .expect("Failed to create client");

    // Clone for background task
    let db_for_task = database.clone();

    // Injecter les services dans le TypeMap
    {
        let mut data = client.data.write().await;
        data.insert::<CommandRegistryKey>(registry);
        data.insert::<Database>(database);
        data.insert::<ClaudeClientKey>(claude_client);
    }

    info!("🚀 Starting bot...");

    // Get HTTP client for background task
    let http = client.http.clone();

    // Spawn reminder check background task
    tokio::spawn(async move {
        // Wait a bit for the bot to fully connect
        tokio::time::sleep(Duration::from_secs(10)).await;
        reminder_check_task(http, db_for_task).await;
    });

    if let Err(e) = client.start().await {
        error!("Client error: {:?}", e);
    }
}

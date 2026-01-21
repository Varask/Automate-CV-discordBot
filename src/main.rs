mod commands;
mod db;
mod services;

use commands::{
    ApplyJobCommand, ClearAllCvsCommand, CommandRegistry, DeleteCvCommand,
    GenerateCoverLetterCommand, GenerateMarketAnalysisCommand, GenerateResumeCommand,
    GetCvCommand, HelpCommand, ListCvsCommand, ListMyCvsCommand, MyStatsCommand,
    SendCvCommand, StatusCommand, SynthesizeOfferCommand, UpdateStatusCommand,
};
use db::Database;
use services::McpClient;
use serenity::all::{GatewayIntents, GuildId, Interaction};
use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use std::env;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Clé pour stocker le registre de commandes dans le TypeMap de Serenity
struct CommandRegistryKey;

impl TypeMapKey for CommandRegistryKey {
    type Value = Arc<CommandRegistry>;
}

/// Clé pour stocker le client MCP dans le TypeMap de Serenity
pub struct McpClientKey;

impl TypeMapKey for McpClientKey {
    type Value = Arc<McpClient>;
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
        if let Interaction::Command(cmd) = interaction {
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

    // Initialiser le client MCP (Claude Code)
    let mcp_client = Arc::new(McpClient::from_env());

    // Tenter de se connecter au serveur MCP
    match mcp_client.connect().await {
        Ok(_) => info!("🤖 Connected to Claude Code MCP server"),
        Err(e) => warn!("⚠️ MCP connection failed (will retry on demand): {}", e),
    }

    let token = env::var("DISCORD_BOT_TOKEN").expect("Expected DISCORD_BOT_TOKEN in .env");

    // Construire le registre de commandes
    let registry = Arc::new(build_registry());

    // Créer le client Discord
    let mut client = Client::builder(&token, GatewayIntents::empty())
        .event_handler(Handler)
        .await
        .expect("Failed to create client");

    // Injecter les services dans le TypeMap
    {
        let mut data = client.data.write().await;
        data.insert::<CommandRegistryKey>(registry);
        data.insert::<Database>(database);
        data.insert::<McpClientKey>(mcp_client);
    }

    info!("🚀 Starting bot...");

    if let Err(e) = client.start().await {
        error!("Client error: {:?}", e);
    }
}
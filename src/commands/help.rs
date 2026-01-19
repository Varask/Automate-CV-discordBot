use async_trait::async_trait;
use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use super::{CommandError, SlashCommand};

pub struct HelpCommand {
    /// Référence aux descriptions des commandes (injectée à la construction)
    commands_info: Vec<(&'static str, &'static str)>,
}

impl HelpCommand {
    pub fn new(commands_info: Vec<(&'static str, &'static str)>) -> Self {
        Self { commands_info }
    }
}

#[async_trait]
impl SlashCommand for HelpCommand {
    fn name(&self) -> &'static str {
        "help"
    }

    fn description(&self) -> &'static str {
        "Display help information about the bot's commands"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name()).description(self.description())
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        let mut help_text = String::from("**📚 Available Commands:**\n\n");

        for (name, description) in &self.commands_info {
            help_text.push_str(&format!("• **/{name}** — {description}\n"));
        }

        let msg = CreateInteractionResponseMessage::new().content(help_text);
        
        interaction
            .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        Ok(())
    }
}

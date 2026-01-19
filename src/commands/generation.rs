use async_trait::async_trait;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use super::{CommandError, SlashCommand};

// ============================================================================
// SynthesizeOffer Command
// ============================================================================

pub struct SynthesizeOfferCommand;

impl SynthesizeOfferCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SynthesizeOfferCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for SynthesizeOfferCommand {
    fn name(&self) -> &'static str {
        "synthesizeoffer"
    }

    fn description(&self) -> &'static str {
        "Synthesize key information from a job description"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description(self.description())
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "description",
                    "Job description text or URL",
                )
                .required(true),
            )
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        // Acknowledge immédiatement (les opérations AI peuvent être longues)
        defer_response(ctx, interaction).await?;

        let _description = get_string_option(interaction, "description")?;

        // TODO: Appeler le service AI pour synthétiser l'offre
        let response = "🔍 **Job Offer Synthesis**\n\nAI analysis coming soon!";

        followup_response(ctx, interaction, response).await
    }
}

// ============================================================================
// GenerateResume Command
// ============================================================================

pub struct GenerateResumeCommand;

impl GenerateResumeCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GenerateResumeCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for GenerateResumeCommand {
    fn name(&self) -> &'static str {
        "generateresume"
    }

    fn description(&self) -> &'static str {
        "Generate a tailored resume based on job description and your CV"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description(self.description())
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "job_description",
                    "Job description text or URL",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Attachment,
                    "cv",
                    "Your existing CV file",
                )
                .required(true),
            )
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        defer_response(ctx, interaction).await?;

        let _job_description = get_string_option(interaction, "job_description")?;
        // let _cv_attachment = get_attachment_option(interaction, "cv")?;

        // TODO: Générer le CV adapté avec AI
        let response = "📝 **Resume Generation**\n\nAI-powered resume generation coming soon!";

        followup_response(ctx, interaction, response).await
    }
}

// ============================================================================
// GenerateCoverLetter Command
// ============================================================================

pub struct GenerateCoverLetterCommand;

impl GenerateCoverLetterCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GenerateCoverLetterCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for GenerateCoverLetterCommand {
    fn name(&self) -> &'static str {
        "generatecoverletter"
    }

    fn description(&self) -> &'static str {
        "Generate a cover letter based on job description and CV"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description(self.description())
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "job_description",
                    "Job description text or URL",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Attachment,
                    "cv",
                    "Your CV file",
                )
                .required(true),
            )
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        defer_response(ctx, interaction).await?;

        let _job_description = get_string_option(interaction, "job_description")?;

        // TODO: Générer la lettre de motivation avec AI
        let response = "✉️ **Cover Letter Generation**\n\nAI-powered cover letter coming soon!";

        followup_response(ctx, interaction, response).await
    }
}

// ============================================================================
// GenerateMarketAnalysis Command
// ============================================================================

pub struct GenerateMarketAnalysisCommand;

impl GenerateMarketAnalysisCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GenerateMarketAnalysisCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for GenerateMarketAnalysisCommand {
    fn name(&self) -> &'static str {
        "generatemarketanalysis"
    }

    fn description(&self) -> &'static str {
        "Generate a market analysis based on job trends and your skills"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name()).description(self.description())
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        defer_response(ctx, interaction).await?;

        // TODO: Analyser le marché avec AI
        let response = "📊 **Market Analysis**\n\nAI-powered market analysis coming soon!";

        followup_response(ctx, interaction, response).await
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn get_string_option(interaction: &CommandInteraction, name: &str) -> Result<String, CommandError> {
    interaction
        .data
        .options
        .iter()
        .find(|opt| opt.name == name)
        .and_then(|opt| opt.value.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| CommandError::MissingParameter(name.to_string()))
}

async fn defer_response(ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
    interaction
        .defer(&ctx.http)
        .await
        .map_err(|e| CommandError::ResponseFailed(e.to_string()))
}

async fn followup_response(
    ctx: &Context,
    interaction: &CommandInteraction,
    content: &str,
) -> Result<(), CommandError> {
    interaction
        .edit_response(&ctx.http, serenity::all::EditInteractionResponse::new().content(content))
        .await
        .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;
    
    Ok(())
}

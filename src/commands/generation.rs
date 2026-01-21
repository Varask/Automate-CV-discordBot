use async_trait::async_trait;
use serenity::all::{
    Colour, CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::{error, info};

use super::{CommandError, SlashCommand};
use crate::db::Database;
use crate::ClaudeClientKey;

const COLOR_SYNTHESIS: Colour = Colour::from_rgb(46, 204, 113);
const COLOR_SALARY: Colour = Colour::from_rgb(230, 126, 34);

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
        defer_response(ctx, interaction).await?;

        let description = get_string_option(interaction, "description")?;

        // Récupérer le client Claude
        let claude_client = {
            let data = ctx.data.read().await;
            data.get::<ClaudeClientKey>()
                .ok_or_else(|| CommandError::Internal("Claude client not found".to_string()))?
                .clone()
        };

        info!("Synthesizing job offer");

        match claude_client.synthesize_job_offer(&description).await {
            Ok(synthesis) => {
                let mut embed = CreateEmbed::new()
                    .title("📋 SYNTHÈSE DE L'OFFRE")
                    .colour(COLOR_SYNTHESIS)
                    .field("🏢 Entreprise", &synthesis.company, true)
                    .field("💼 Poste", &synthesis.title, true)
                    .field("📍 Lieu", &synthesis.location, true)
                    .field("📝 Contrat", &synthesis.contract_type, true);

                if let Some(salary) = &synthesis.salary_range {
                    embed = embed.field("💰 Salaire", salary, true);
                }

                let requirements = if synthesis.key_requirements.is_empty() {
                    "Non spécifié".to_string()
                } else {
                    synthesis.key_requirements.iter()
                        .map(|r| format!("• {}", r))
                        .collect::<Vec<_>>()
                        .join("\n")
                };

                embed = embed.field("🎯 Compétences clés", requirements, false);
                embed = embed.field("📖 Résumé", &synthesis.summary, false);

                followup_embed(ctx, interaction, embed).await
            }
            Err(e) => {
                error!("Failed to synthesize: {}", e);
                followup_response(ctx, interaction, &format!("❌ Erreur: {}", e)).await
            }
        }
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
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        defer_response(ctx, interaction).await?;

        let job_description = get_string_option(interaction, "job_description")?;
        let user_id = interaction.user.id;

        // Récupérer le client Claude et la DB
        let (claude_client, db) = {
            let data = ctx.data.read().await;
            let claude = data.get::<ClaudeClientKey>()
                .ok_or_else(|| CommandError::Internal("Claude client not found".to_string()))?
                .clone();
            let db = data.get::<Database>()
                .ok_or_else(|| CommandError::Internal("Database not found".to_string()))?
                .clone();
            (claude, db)
        };

        // Récupérer le CV de l'utilisateur
        let user_cv = db.get_active_cv(user_id.get() as i64)
            .map_err(|e| CommandError::Internal(format!("Database error: {}", e)))?;

        let cv_content = match &user_cv {
            Some(cv) => {
                match tokio::fs::read_to_string(&cv.file_path).await {
                    Ok(content) => content,
                    Err(_) => cv.extracted_text.clone().unwrap_or_else(|| "CV non lisible".to_string())
                }
            }
            None => {
                return followup_response(ctx, interaction,
                    "❌ **Aucun CV trouvé**\n\nUtilisez `/sendcv` pour uploader votre CV d'abord."
                ).await;
            }
        };

        info!("Generating resume for user {}", user_id);

        // 1. Synthétiser l'offre
        let synthesis = match claude_client.synthesize_job_offer(&job_description).await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to synthesize: {}", e);
                return followup_response(ctx, interaction, &format!("❌ Erreur de synthèse: {}", e)).await;
            }
        };

        // 2. Matcher les skills
        let skills_match = match claude_client.match_skills(&job_description, &cv_content).await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to match skills: {}", e);
                return followup_response(ctx, interaction, &format!("❌ Erreur d'analyse: {}", e)).await;
            }
        };

        // 3. Générer le CV
        match claude_client.generate_tailored_cv(&cv_content, &synthesis, &skills_match).await {
            Ok(generated) => {
                let mut embed = CreateEmbed::new()
                    .title("📝 CV PERSONNALISÉ GÉNÉRÉ")
                    .colour(Colour::from_rgb(52, 152, 219))
                    .field("🎯 Poste ciblé", format!("{} chez {}", synthesis.title, synthesis.company), false)
                    .field("📊 Score de matching", format!("{}%", skills_match.match_score), true)
                    .field("📝 Résumé", &generated.summary, false);

                if !generated.adaptations.is_empty() {
                    let adaptations = generated.adaptations.iter()
                        .take(5)
                        .map(|a| format!("• {}", a))
                        .collect::<Vec<_>>()
                        .join("\n");
                    embed = embed.field("✨ Adaptations", adaptations, false);
                }

                followup_embed(ctx, interaction, embed).await
            }
            Err(e) => {
                error!("Failed to generate CV: {}", e);
                followup_response(ctx, interaction, &format!("❌ Erreur de génération: {}", e)).await
            }
        }
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
        "Generate a cover letter based on job description and your stored CV"
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
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        defer_response(ctx, interaction).await?;

        let job_description = get_string_option(interaction, "job_description")?;
        let user_id = interaction.user.id;

        // Récupérer Claude et DB
        let (claude_client, db) = {
            let data = ctx.data.read().await;
            let claude = data.get::<ClaudeClientKey>()
                .ok_or_else(|| CommandError::Internal("Claude client not found".to_string()))?
                .clone();
            let db = data.get::<Database>()
                .ok_or_else(|| CommandError::Internal("Database not found".to_string()))?
                .clone();
            (claude, db)
        };

        // Récupérer le CV
        let user_cv = db.get_active_cv(user_id.get() as i64)
            .map_err(|e| CommandError::Internal(format!("Database error: {}", e)))?;

        let cv_content = match &user_cv {
            Some(cv) => {
                tokio::fs::read_to_string(&cv.file_path).await
                    .unwrap_or_else(|_| cv.extracted_text.clone().unwrap_or_default())
            }
            None => String::new()
        };

        info!("Generating cover letter for user {}", user_id);

        // Prompt pour générer la lettre de motivation
        let prompt = format!(
            "Génère une lettre de motivation professionnelle en français pour cette offre d'emploi. \
            Retourne UNIQUEMENT le texte de la lettre, sans JSON.\n\n\
            Offre:\n{}\n\n\
            CV du candidat:\n{}",
            job_description,
            if cv_content.is_empty() { "Non fourni" } else { &cv_content }
        );

        match claude_client.prompt(&prompt).await {
            Ok(letter) => {
                // Discord limite les messages à 2000 caractères
                let truncated = if letter.len() > 1900 {
                    format!("{}...\n\n_[Tronqué - lettre complète disponible sur demande]_", &letter[..1900])
                } else {
                    letter
                };

                let embed = CreateEmbed::new()
                    .title("✉️ LETTRE DE MOTIVATION")
                    .colour(Colour::from_rgb(155, 89, 182))
                    .description(truncated);

                followup_embed(ctx, interaction, embed).await
            }
            Err(e) => {
                error!("Failed to generate cover letter: {}", e);
                followup_response(ctx, interaction, &format!("❌ Erreur: {}", e)).await
            }
        }
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

        let user_id = interaction.user.id;

        // Récupérer Claude et DB
        let (claude_client, db) = {
            let data = ctx.data.read().await;
            let claude = data.get::<ClaudeClientKey>()
                .ok_or_else(|| CommandError::Internal("Claude client not found".to_string()))?
                .clone();
            let db = data.get::<Database>()
                .ok_or_else(|| CommandError::Internal("Database not found".to_string()))?
                .clone();
            (claude, db)
        };

        // Récupérer le CV pour l'analyse de marché
        let user_cv = db.get_active_cv(user_id.get() as i64)
            .map_err(|e| CommandError::Internal(format!("Database error: {}", e)))?;

        let cv_content = match &user_cv {
            Some(cv) => {
                tokio::fs::read_to_string(&cv.file_path).await
                    .unwrap_or_else(|_| cv.extracted_text.clone().unwrap_or_default())
            }
            None => {
                return followup_response(ctx, interaction,
                    "❌ **Aucun CV trouvé**\n\nUtilisez `/sendcv` pour uploader votre CV d'abord."
                ).await;
            }
        };

        info!("Generating market analysis for user {}", user_id);

        let prompt = format!(
            "Analyse le marché de l'emploi basé sur ce CV. Retourne un JSON:\n\
            {{\n\
                \"profile_summary\": \"résumé du profil\",\n\
                \"key_skills\": [\"skill1\", \"skill2\"],\n\
                \"market_demand\": \"haute/moyenne/basse\",\n\
                \"salary_range\": \"fourchette salariale estimée\",\n\
                \"trending_skills\": [\"skill à développer\"],\n\
                \"job_titles\": [\"postes correspondants\"],\n\
                \"recommendations\": [\"conseil 1\"]\n\
            }}\n\nCV:\n{}",
            cv_content
        );

        match claude_client.prompt(&prompt).await {
            Ok(response) => {
                // Parser le JSON ou afficher brut
                let embed = CreateEmbed::new()
                    .title("📊 ANALYSE DE MARCHÉ")
                    .colour(Colour::from_rgb(52, 73, 94))
                    .description(if response.len() > 1900 {
                        format!("{}...", &response[..1900])
                    } else {
                        response
                    });

                followup_embed(ctx, interaction, embed).await
            }
            Err(e) => {
                error!("Failed to analyze market: {}", e);
                followup_response(ctx, interaction, &format!("❌ Erreur: {}", e)).await
            }
        }
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

async fn followup_embed(
    ctx: &Context,
    interaction: &CommandInteraction,
    embed: CreateEmbed,
) -> Result<(), CommandError> {
    interaction
        .edit_response(&ctx.http, serenity::all::EditInteractionResponse::new().embed(embed))
        .await
        .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

    Ok(())
}

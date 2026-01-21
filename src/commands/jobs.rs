use async_trait::async_trait;
use serenity::all::{
    Colour, CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    EditInteractionResponse,
};
use tracing::{error, info};

use super::{CommandError, SlashCommand};
use crate::services::{JobSynthesis, SalaryAnalysis, SkillsMatch};
use crate::ClaudeClientKey;

// Couleurs des embeds
const COLOR_SYNTHESIS: Colour = Colour::from_rgb(46, 204, 113);   // Vert
const COLOR_SKILLS: Colour = Colour::from_rgb(241, 196, 15);      // Jaune
const COLOR_SALARY: Colour = Colour::from_rgb(230, 126, 34);      // Orange
const COLOR_CV: Colour = Colour::from_rgb(52, 152, 219);          // Bleu

// ============================================================================
// ApplyJob Command
// Combines: job synthesis + CV generation + salary analysis
// ============================================================================

pub struct ApplyJobCommand;

impl ApplyJobCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ApplyJobCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for ApplyJobCommand {
    fn name(&self) -> &'static str {
        "applyjob"
    }

    fn description(&self) -> &'static str {
        "Apply to a job: generates synthesis, tailored CV, and salary analysis"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description(self.description())
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "description",
                    "Job description (paste the full text)",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "url",
                    "Job posting URL (optional)",
                )
                .required(false),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "company",
                    "Company name (optional, will try to extract)",
                )
                .required(false),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "title",
                    "Job title (optional, will try to extract)",
                )
                .required(false),
            )
    }

    async fn execute(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> Result<(), CommandError> {
        // Defer - this will take time (AI processing)
        interaction
            .defer(&ctx.http)
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        let user_id = interaction.user.id;

        // Get options
        let job_description = get_string_option(interaction, "description")?;
        let _job_url = get_optional_string_option(interaction, "url");
        let _company = get_optional_string_option(interaction, "company");
        let _title = get_optional_string_option(interaction, "title");

        // Récupérer le client Claude
        let claude_client = {
            let data = ctx.data.read().await;
            data.get::<ClaudeClientKey>()
                .ok_or_else(|| CommandError::Internal("Claude client not found".to_string()))?
                .clone()
        };

        info!("Processing job application for user {}", user_id);

        // Envoyer un message initial
        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .content("🔄 **Analyse en cours...**\n\n⏳ Synthèse de l'offre..."),
            )
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        // 1. Synthétiser l'offre d'emploi
        let synthesis = match claude_client.synthesize_job_offer(&job_description).await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to synthesize job offer: {}", e);
                return send_error_response(
                    ctx,
                    interaction,
                    &format!("Erreur lors de la synthèse: {}", e),
                )
                .await;
            }
        };

        // Envoyer l'embed de synthèse (Vert)
        let synthesis_embed = build_synthesis_embed(&synthesis);
        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .content("✅ Synthèse terminée\n⏳ Analyse des compétences...")
                    .embed(synthesis_embed),
            )
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        // 2. Analyse des compétences (pour l'instant sans CV, on utilisera un placeholder)
        // TODO: Récupérer le CV de l'utilisateur depuis la DB
        let cv_placeholder = "CV non fourni - analyse basée sur l'offre uniquement";

        let skills_match = match claude_client
            .match_skills(&job_description, cv_placeholder)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to match skills: {}", e);
                // Continuer avec des valeurs par défaut
                SkillsMatch {
                    match_score: 0,
                    matched_skills: vec![],
                    missing_skills: vec![],
                    highlights: vec!["Uploadez votre CV pour une analyse personnalisée".to_string()],
                    recommendations: vec![],
                }
            }
        };

        // Créer un channel followup pour envoyer plusieurs embeds
        let skills_embed = build_skills_embed(&skills_match);
        interaction
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new().embed(skills_embed),
            )
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        // 3. Analyse salariale
        let salary_analysis = match claude_client
            .analyze_salary(&job_description, Some(&synthesis.location))
            .await
        {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to analyze salary: {}", e);
                SalaryAnalysis {
                    offered_min: None,
                    offered_max: None,
                    market_low: 0,
                    market_median: 0,
                    market_high: 0,
                    currency: "EUR".to_string(),
                    analysis: format!("Analyse non disponible: {}", e),
                    negotiation_tips: vec![],
                }
            }
        };

        let salary_embed = build_salary_embed(&salary_analysis);
        interaction
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new().embed(salary_embed),
            )
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        // 4. Message final (CV sera généré quand l'utilisateur aura uploadé son CV)
        let final_embed = CreateEmbed::new()
            .title("📄 Génération de CV")
            .description(
                "Pour générer un CV personnalisé, utilisez `/sendcv` pour uploader votre CV de base, \
                puis relancez `/applyjob`.",
            )
            .colour(COLOR_CV)
            .field("Prochaines étapes", "1. `/sendcv` - Uploader votre CV\n2. `/applyjob` - Relancer l'analyse\n3. Télécharger votre CV personnalisé", false);

        interaction
            .create_followup(
                &ctx.http,
                serenity::all::CreateInteractionResponseFollowup::new().embed(final_embed),
            )
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        info!("Job application analysis completed for user {}", user_id);

        Ok(())
    }
}

// ============================================================================
// Embed builders
// ============================================================================

fn build_synthesis_embed(synthesis: &JobSynthesis) -> CreateEmbed {
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
        synthesis
            .key_requirements
            .iter()
            .map(|r| format!("• {}", r))
            .collect::<Vec<_>>()
            .join("\n")
    };

    embed = embed.field("🎯 Compétences clés", requirements, false);
    embed = embed.field("📖 Résumé", &synthesis.summary, false);

    embed
}

fn build_skills_embed(skills: &SkillsMatch) -> CreateEmbed {
    let score_bar = build_progress_bar(skills.match_score, 100);

    let mut embed = CreateEmbed::new()
        .title("🎯 ANALYSE DE COMPATIBILITÉ")
        .colour(COLOR_SKILLS)
        .field(
            "Score de matching",
            format!("{} **{}%**", score_bar, skills.match_score),
            false,
        );

    // Compétences matchées
    if !skills.matched_skills.is_empty() {
        let matched = skills
            .matched_skills
            .iter()
            .take(5)
            .map(|s| {
                let icon = if s.is_match { "✅" } else { "⚠️" };
                format!("{} **{}**: {} → Requis: {}", icon, s.skill, s.cv_level, s.required)
            })
            .collect::<Vec<_>>()
            .join("\n");
        embed = embed.field("✅ Compétences matchées", matched, false);
    }

    // Compétences manquantes
    if !skills.missing_skills.is_empty() {
        let missing = skills
            .missing_skills
            .iter()
            .take(5)
            .map(|s| format!("❌ **{}** ({})", s.skill, s.importance))
            .collect::<Vec<_>>()
            .join("\n");
        embed = embed.field("❌ Compétences manquantes", missing, false);
    }

    // Points forts
    if !skills.highlights.is_empty() {
        let highlights = skills
            .highlights
            .iter()
            .take(3)
            .map(|h| format!("⭐ {}", h))
            .collect::<Vec<_>>()
            .join("\n");
        embed = embed.field("⭐ Points forts à mettre en avant", highlights, false);
    }

    embed
}

fn build_salary_embed(salary: &SalaryAnalysis) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("💰 ANALYSE SALARIALE")
        .colour(COLOR_SALARY);

    // Salaire proposé
    if salary.offered_min.is_some() || salary.offered_max.is_some() {
        let offered = match (salary.offered_min, salary.offered_max) {
            (Some(min), Some(max)) => format!("{}k€ - {}k€", min / 1000, max / 1000),
            (Some(min), None) => format!("À partir de {}k€", min / 1000),
            (None, Some(max)) => format!("Jusqu'à {}k€", max / 1000),
            _ => "Non spécifié".to_string(),
        };
        embed = embed.field("💵 Salaire annoncé", offered, false);
    }

    // Fourchette marché
    if salary.market_median > 0 {
        let market = format!(
            "📉 Bas: **{}k€**\n📊 Médian: **{}k€**\n📈 Haut: **{}k€**",
            salary.market_low / 1000,
            salary.market_median / 1000,
            salary.market_high / 1000
        );
        embed = embed.field(
            format!("📊 Marché ({}) ", salary.currency),
            market,
            false,
        );
    }

    if !salary.analysis.is_empty() {
        embed = embed.field("📝 Analyse", &salary.analysis, false);
    }

    // Conseils de négociation
    if !salary.negotiation_tips.is_empty() {
        let tips = salary
            .negotiation_tips
            .iter()
            .take(3)
            .map(|t| format!("💡 {}", t))
            .collect::<Vec<_>>()
            .join("\n");
        embed = embed.field("💡 Conseils de négociation", tips, false);
    }

    embed
}

fn build_progress_bar(value: u32, max: u32) -> String {
    let percentage = (value as f32 / max as f32 * 10.0).round() as usize;
    let filled = "█".repeat(percentage.min(10));
    let empty = "░".repeat(10 - percentage.min(10));
    format!("{}{}", filled, empty)
}

async fn send_error_response(
    ctx: &Context,
    interaction: &CommandInteraction,
    message: &str,
) -> Result<(), CommandError> {
    interaction
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(format!("❌ **Erreur**: {}", message)),
        )
        .await
        .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;
    Ok(())
}

// ============================================================================
// Status Command
// ============================================================================

pub struct StatusCommand;

impl StatusCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StatusCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for StatusCommand {
    fn name(&self) -> &'static str {
        "status"
    }

    fn description(&self) -> &'static str {
        "View your job application statuses"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description(self.description())
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "filter", "Filter by status")
                    .required(false)
                    .add_string_choice("All", "all")
                    .add_string_choice("Generated", "generated")
                    .add_string_choice("Applied", "applied")
                    .add_string_choice("Interview", "interview")
                    .add_string_choice("Offer", "offer")
                    .add_string_choice("Rejected", "rejected")
                    .add_string_choice("Accepted", "accepted"),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "limit",
                    "Number of results (default: 10)",
                )
                .required(false)
                .min_int_value(1)
                .max_int_value(25),
            )
    }

    async fn execute(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> Result<(), CommandError> {
        let _user_id = interaction.user.id;
        let filter = get_optional_string_option(interaction, "filter").unwrap_or_else(|| "all".to_string());
        let limit = get_optional_int_option(interaction, "limit").unwrap_or(10);

        let response = format!(
            "📊 **Your Applications** (filter: {}, limit: {})\n\n\
            _Aucune candidature enregistrée_\n\n\
            Utilisez `/applyjob` pour analyser une offre d'emploi.",
            filter, limit
        );

        send_response(ctx, interaction, &response).await
    }
}

// ============================================================================
// UpdateStatus Command
// ============================================================================

pub struct UpdateStatusCommand;

impl UpdateStatusCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for UpdateStatusCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for UpdateStatusCommand {
    fn name(&self) -> &'static str {
        "updatestatus"
    }

    fn description(&self) -> &'static str {
        "Update the status of a job application"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description(self.description())
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "application_id",
                    "Application ID (from /status)",
                )
                .required(true)
                .min_int_value(1),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "status", "New status")
                    .required(true)
                    .add_string_choice("Applied", "applied")
                    .add_string_choice("Interview Scheduled", "interview")
                    .add_string_choice("Offer Received", "offer")
                    .add_string_choice("Rejected", "rejected")
                    .add_string_choice("Accepted", "accepted"),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "note", "Add a note (optional)")
                    .required(false),
            )
    }

    async fn execute(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> Result<(), CommandError> {
        let application_id = get_int_option(interaction, "application_id")?;
        let new_status = get_string_option(interaction, "status")?;
        let note = get_optional_string_option(interaction, "note");

        let status_emoji = match new_status.as_str() {
            "applied" => "🟡",
            "interview" => "🟢",
            "offer" => "🎉",
            "rejected" => "🔴",
            "accepted" => "✅",
            _ => "⚪",
        };

        let response = format!(
            "{} **Status Updated**\n\n\
            Application #{} → **{}**\n\
            {}",
            status_emoji,
            application_id,
            new_status,
            note.map(|n| format!("📝 Note: {}", n)).unwrap_or_default()
        );

        send_response(ctx, interaction, &response).await
    }
}

// ============================================================================
// MyStats Command
// ============================================================================

pub struct MyStatsCommand;

impl MyStatsCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MyStatsCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for MyStatsCommand {
    fn name(&self) -> &'static str {
        "mystats"
    }

    fn description(&self) -> &'static str {
        "View your application statistics"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name()).description(self.description())
    }

    async fn execute(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> Result<(), CommandError> {
        let user_id = interaction.user.id;

        let response = format!(
            "📈 **Your Statistics** <@{}>\n\n\
            _Aucune statistique disponible_\n\n\
            Utilisez `/applyjob` pour commencer à tracker vos candidatures.",
            user_id
        );

        send_response(ctx, interaction, &response).await
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

fn get_optional_string_option(interaction: &CommandInteraction, name: &str) -> Option<String> {
    interaction
        .data
        .options
        .iter()
        .find(|opt| opt.name == name)
        .and_then(|opt| opt.value.as_str())
        .map(|s| s.to_string())
}

fn get_int_option(interaction: &CommandInteraction, name: &str) -> Result<i64, CommandError> {
    interaction
        .data
        .options
        .iter()
        .find(|opt| opt.name == name)
        .and_then(|opt| opt.value.as_i64())
        .ok_or_else(|| CommandError::MissingParameter(name.to_string()))
}

fn get_optional_int_option(interaction: &CommandInteraction, name: &str) -> Option<i64> {
    interaction
        .data
        .options
        .iter()
        .find(|opt| opt.name == name)
        .and_then(|opt| opt.value.as_i64())
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

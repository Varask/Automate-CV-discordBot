use async_trait::async_trait;
use serenity::all::{
    ButtonStyle, ChannelType, Colour, CommandInteraction, CommandOptionType, Context,
    CreateActionRow, CreateButton, CreateCommand, CreateCommandOption, CreateAttachment,
    CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
    CreateThread, EditInteractionResponse,
};
use tracing::{error, info, warn};

use super::{CommandError, SlashCommand};
use crate::db::Database;
use crate::services::{JobSynthesis, SalaryAnalysis, SkillsMatch};
use crate::ClaudeClientKey;

// Couleurs des embeds
const COLOR_SYNTHESIS: Colour = Colour::from_rgb(46, 204, 113);   // Vert
const COLOR_SKILLS: Colour = Colour::from_rgb(241, 196, 15);      // Jaune
const COLOR_SALARY: Colour = Colour::from_rgb(230, 126, 34);      // Orange
const COLOR_CV: Colour = Colour::from_rgb(52, 152, 219);          // Bleu
const COLOR_TRACKING: Colour = Colour::from_rgb(155, 89, 182);    // Violet

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
                .required(false),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Attachment,
                    "description_file",
                    "Job description file (TXT)",
                )
                .required(false),
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
        let channel_id = interaction.channel_id;

        // Get options
        let text_description = get_optional_string_option(interaction, "description");
        let _job_url = get_optional_string_option(interaction, "url");
        let _company = get_optional_string_option(interaction, "company");
        let _title = get_optional_string_option(interaction, "title");

        // Check for file attachment
        let file_description = get_optional_attachment_content(interaction, "description_file").await;

        // Determine job description: file takes priority, then text
        let job_description = match (file_description, text_description) {
            (Ok(Some(content)), _) => {
                info!("Using job description from file for user {}", user_id);
                content
            }
            (_, Some(text)) => {
                info!("Using job description from text for user {}", user_id);
                text
            }
            (Err(e), None) => {
                return send_error_response(
                    ctx,
                    interaction,
                    &format!("Erreur lors de la lecture du fichier: {}", e),
                )
                .await;
            }
            (Ok(None), None) => {
                return send_error_response(
                    ctx,
                    interaction,
                    "Veuillez fournir une description de l'offre (texte ou fichier).",
                )
                .await;
            }
        };

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

        info!("Processing job application for user {}", user_id);

        // Envoyer un embed de suivi initial dans le canal principal
        let initial_tracking_embed = build_tracking_embed_progress("Synthèse de l'offre...", None, None);
        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().embed(initial_tracking_embed),
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

        // Créer le thread pour les résultats détaillés
        let thread_name = format!("📋 {} - {}", synthesis.company, synthesis.title);
        let thread_name = if thread_name.len() > 100 {
            format!("{}...", &thread_name[..97])
        } else {
            thread_name
        };

        // 2. Récupérer le CV de l'utilisateur depuis la DB
        let user_cv = db.get_active_cv(user_id.get() as i64)
            .map_err(|e| CommandError::Internal(format!("Database error: {}", e)))?;

        // Sauvegarder la candidature en DB
        let cv_id = user_cv.as_ref().map(|cv| cv.id).unwrap_or(0);
        let application_id = db
            .create_application(
                user_id.get() as i64,
                cv_id,
                Some(&synthesis.title),
                Some(&synthesis.company),
                Some(&synthesis.location),
                None, // job_url
                &job_description,
            )
            .map_err(|e| CommandError::Internal(format!("Failed to save application: {}", e)))?;

        info!("Created application {} for user {}", application_id, user_id);

        // Créer le thread pour les résultats détaillés
        let thread_name = format!("📋 {} - {}", synthesis.company, synthesis.title);
        let thread_name = if thread_name.len() > 100 {
            format!("{}...", &thread_name[..97])
        } else {
            thread_name
        };

        let thread = channel_id
            .create_thread(
                &ctx.http,
                CreateThread::new(thread_name.clone())
                    .kind(ChannelType::PublicThread)
                    .auto_archive_duration(serenity::all::AutoArchiveDuration::OneDay),
            )
            .await
            .map_err(|e| CommandError::Internal(format!("Failed to create thread: {}", e)))?;

        info!("Created thread {} for job application", thread.id);

        // Sauvegarder le thread_id en DB
        if let Err(e) = db.update_application_thread(application_id, thread.id.get() as i64) {
            warn!("Failed to save thread_id: {}", e);
        }

        // Mettre à jour l'embed de suivi avec le lien vers le thread
        let tracking_embed = build_tracking_embed_progress(
            "Analyse des compétences...",
            Some(&synthesis),
            Some(thread.id.get()),
        );
        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().embed(tracking_embed),
            )
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        // Envoyer l'embed de synthèse dans le thread
        let synthesis_embed = build_synthesis_embed(&synthesis);
        thread
            .send_message(&ctx.http, CreateMessage::new().embed(synthesis_embed))
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        let cv_content = match &user_cv {
            Some(cv) => {
                if let Some(ref extracted) = cv.extracted_text {
                    if !extracted.is_empty() {
                        info!("Using extracted text for CV {} (user {})", cv.id, user_id);
                        extracted.clone()
                    } else {
                        warn!("Extracted text is empty for CV {}", cv.id);
                        format!("CV: {} (texte non disponible - réuploadez votre CV)", cv.original_name)
                    }
                } else {
                    match tokio::fs::read_to_string(&cv.file_path).await {
                        Ok(content) => {
                            info!("Read CV file directly for user {}", user_id);
                            content
                        }
                        Err(_) => {
                            warn!("No extracted text and file not readable for CV {}", cv.id);
                            format!("CV: {} (texte non extrait - réuploadez votre CV avec /sendcv)", cv.original_name)
                        }
                    }
                }
            }
            None => {
                info!("No CV found for user {}", user_id);
                "CV non fourni - analyse basée sur l'offre uniquement".to_string()
            }
        };

        let has_cv = user_cv.is_some();

        // Analyse des compétences
        let skills_match = match claude_client
            .match_skills(&job_description, &cv_content)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to match skills: {}", e);
                let default_highlight = if has_cv {
                    "Analyse en cours...".to_string()
                } else {
                    "Uploadez votre CV avec `/sendcv` pour une analyse personnalisée".to_string()
                };
                SkillsMatch {
                    match_score: 0,
                    matched_skills: vec![],
                    missing_skills: vec![],
                    highlights: vec![default_highlight],
                    recommendations: vec![],
                }
            }
        };

        // Mettre à jour le tracking
        let tracking_embed = build_tracking_embed_progress(
            "Analyse salariale...",
            Some(&synthesis),
            Some(thread.id.get()),
        );
        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().embed(tracking_embed),
            )
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        // Envoyer l'embed des compétences dans le thread
        let skills_embed = build_skills_embed(&skills_match);
        thread
            .send_message(&ctx.http, CreateMessage::new().embed(skills_embed))
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

        // Envoyer l'embed salarial dans le thread
        let salary_embed = build_salary_embed(&salary_analysis);
        thread
            .send_message(&ctx.http, CreateMessage::new().embed(salary_embed))
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        // 4. Génération de CV personnalisé si CV disponible
        let cv_generated = if has_cv {
            // Mettre à jour le tracking
            let tracking_embed = build_tracking_embed_progress(
                "Génération du CV personnalisé...",
                Some(&synthesis),
                Some(thread.id.get()),
            );
            interaction
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().embed(tracking_embed),
                )
                .await
                .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

            match claude_client
                .generate_tailored_cv(&cv_content, &synthesis, &skills_match)
                .await
            {
                Ok(generated_cv) => {
                    let mut embed = CreateEmbed::new()
                        .title("📄 CV PERSONNALISÉ GÉNÉRÉ")
                        .colour(COLOR_CV)
                        .field("📝 Résumé des adaptations", &generated_cv.summary, false);

                    if !generated_cv.adaptations.is_empty() {
                        let adaptations = generated_cv
                            .adaptations
                            .iter()
                            .take(5)
                            .map(|a| format!("• {}", a))
                            .collect::<Vec<_>>()
                            .join("\n");
                        embed = embed.field("✨ Modifications apportées", adaptations, false);
                    }

                    let cv_text = generated_cv.get_content();
                    let username = &interaction.user.name;

                    match claude_client
                        .generate_pdf(cv_text, username, &synthesis.title, &synthesis.company)
                        .await
                    {
                        Ok(pdf_bytes) => {
                            let safe_title = synthesis.title
                                .chars()
                                .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
                                .collect::<String>()
                                .replace(' ', "_");
                            let filename = format!("CV_{}_{}.pdf", username, safe_title);
                            let attachment = CreateAttachment::bytes(pdf_bytes, &filename);

                            embed = embed.field(
                                "📥 Téléchargement",
                                "✅ PDF généré et joint ci-dessous!",
                                false,
                            );

                            thread
                                .send_message(
                                    &ctx.http,
                                    CreateMessage::new().embed(embed).add_file(attachment),
                                )
                                .await
                                .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;
                            true
                        }
                        Err(e) => {
                            warn!("Failed to generate PDF: {}", e);
                            embed = embed.field(
                                "📥 Téléchargement",
                                format!("⚠️ Génération PDF échouée: {}", e),
                                false,
                            );

                            thread
                                .send_message(&ctx.http, CreateMessage::new().embed(embed))
                                .await
                                .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;
                            true
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to generate tailored CV: {}", e);
                    let embed = CreateEmbed::new()
                        .title("📄 Génération de CV")
                        .description(format!("Erreur lors de la génération: {}", e))
                        .colour(COLOR_CV)
                        .field(
                            "💡 Conseil",
                            "Réessayez avec `/applyjob` ou vérifiez que votre CV est bien uploadé.",
                            false,
                        );

                    thread
                        .send_message(&ctx.http, CreateMessage::new().embed(embed))
                        .await
                        .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;
                    false
                }
            }
        } else {
            let embed = CreateEmbed::new()
                .title("📄 Génération de CV")
                .description("Pour générer un CV personnalisé, uploadez d'abord votre CV de base.")
                .colour(COLOR_CV)
                .field(
                    "Prochaines étapes",
                    "1. `/sendcv` - Uploader votre CV\n2. `/applyjob` - Relancer l'analyse\n3. Télécharger votre CV personnalisé",
                    false,
                );

            thread
                .send_message(&ctx.http, CreateMessage::new().embed(embed))
                .await
                .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;
            false
        };

        // Mettre à jour l'analyse en DB
        if let Err(e) = db.update_application_analysis(
            application_id,
            &synthesis.summary,
            &serde_json::to_string(&synthesis.key_requirements).unwrap_or_default(),
            &serde_json::to_string(&skills_match.matched_skills).unwrap_or_default(),
            &serde_json::to_string(&skills_match.missing_skills).unwrap_or_default(),
            skills_match.match_score as i32,
        ) {
            warn!("Failed to update application analysis: {}", e);
        }

        // Mettre à jour l'embed de suivi final dans le canal principal avec les boutons
        let final_tracking_embed = build_tracking_embed_complete(
            &synthesis,
            skills_match.match_score,
            has_cv,
            cv_generated,
            thread.id.get(),
            application_id,
            "generated",
        );
        let action_rows = build_status_buttons(application_id, "generated");
        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .embed(final_tracking_embed)
                    .components(action_rows),
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

fn build_tracking_embed_progress(
    current_step: &str,
    synthesis: Option<&JobSynthesis>,
    thread_id: Option<u64>,
) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("🔄 ANALYSE EN COURS")
        .colour(COLOR_TRACKING);

    if let Some(s) = synthesis {
        embed = embed
            .field("🏢 Entreprise", &s.company, true)
            .field("💼 Poste", &s.title, true);
    }

    embed = embed.field("⏳ Étape actuelle", current_step, false);

    if let Some(tid) = thread_id {
        embed = embed.field(
            "📋 Détails",
            format!("Consultez le thread <#{}> pour les résultats détaillés", tid),
            false,
        );
    }

    embed
}

fn build_tracking_embed_complete(
    synthesis: &JobSynthesis,
    match_score: u32,
    has_cv: bool,
    cv_generated: bool,
    thread_id: u64,
    application_id: i64,
    status: &str,
) -> CreateEmbed {
    let score_bar = build_progress_bar(match_score, 100);
    let score_emoji = if match_score >= 70 {
        "🟢"
    } else if match_score >= 40 {
        "🟡"
    } else {
        "🔴"
    };

    let cv_status = if cv_generated {
        "✅ CV personnalisé généré"
    } else if has_cv {
        "⚠️ Erreur de génération"
    } else {
        "❌ Aucun CV (utilisez `/sendcv`)"
    };

    let status_display = get_status_display(status);

    CreateEmbed::new()
        .title("📊 SUIVI DE CANDIDATURE")
        .colour(COLOR_TRACKING)
        .field("🏢 Entreprise", &synthesis.company, true)
        .field("💼 Poste", &synthesis.title, true)
        .field("📍 Lieu", &synthesis.location, true)
        .field(
            "🎯 Score de compatibilité",
            format!("{} {} **{}%**", score_emoji, score_bar, match_score),
            false,
        )
        .field("📄 CV", cv_status, true)
        .field("📌 Statut", status_display, true)
        .field(
            "📋 Résultats détaillés",
            format!("👉 <#{}>", thread_id),
            false,
        )
        .footer(serenity::all::CreateEmbedFooter::new(format!("ID: {}", application_id)))
}

fn get_status_display(status: &str) -> &'static str {
    match status {
        "generated" => "📝 Générée",
        "applied" => "📤 Postulée",
        "interview" => "🗓️ Entretien",
        "offer" => "🎉 Offre reçue",
        "rejected" => "❌ Refusée",
        "accepted" => "✅ Acceptée",
        _ => "❓ Inconnu",
    }
}

fn build_status_buttons(application_id: i64, current_status: &str) -> Vec<CreateActionRow> {
    let buttons_row1 = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("status_{}_{}", application_id, "applied"))
            .label("📤 Postulée")
            .style(if current_status == "applied" {
                ButtonStyle::Success
            } else {
                ButtonStyle::Secondary
            })
            .disabled(current_status == "applied"),
        CreateButton::new(format!("status_{}_{}", application_id, "interview"))
            .label("🗓️ Entretien")
            .style(if current_status == "interview" {
                ButtonStyle::Success
            } else {
                ButtonStyle::Primary
            })
            .disabled(current_status == "interview"),
        CreateButton::new(format!("status_{}_{}", application_id, "offer"))
            .label("🎉 Offre")
            .style(if current_status == "offer" {
                ButtonStyle::Success
            } else {
                ButtonStyle::Primary
            })
            .disabled(current_status == "offer"),
    ]);

    let buttons_row2 = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("status_{}_{}", application_id, "accepted"))
            .label("✅ Acceptée")
            .style(if current_status == "accepted" {
                ButtonStyle::Success
            } else {
                ButtonStyle::Success
            })
            .disabled(current_status == "accepted"),
        CreateButton::new(format!("status_{}_{}", application_id, "rejected"))
            .label("❌ Refusée")
            .style(if current_status == "rejected" {
                ButtonStyle::Danger
            } else {
                ButtonStyle::Danger
            })
            .disabled(current_status == "rejected"),
    ]);

    vec![buttons_row1, buttons_row2]
}

/// Reconstruit l'embed de suivi à partir d'une application existante
pub fn rebuild_tracking_embed_from_status(
    company: &str,
    title: &str,
    location: &str,
    match_score: u32,
    has_cv: bool,
    thread_id: Option<u64>,
    application_id: i64,
    status: &str,
) -> CreateEmbed {
    let score_bar = build_progress_bar(match_score, 100);
    let score_emoji = if match_score >= 70 {
        "🟢"
    } else if match_score >= 40 {
        "🟡"
    } else {
        "🔴"
    };

    let cv_status = if has_cv {
        "✅ CV personnalisé"
    } else {
        "❌ Aucun CV"
    };

    let status_display = get_status_display(status);

    let mut embed = CreateEmbed::new()
        .title("📊 SUIVI DE CANDIDATURE")
        .colour(COLOR_TRACKING)
        .field("🏢 Entreprise", company, true)
        .field("💼 Poste", title, true)
        .field("📍 Lieu", location, true)
        .field(
            "🎯 Score de compatibilité",
            format!("{} {} **{}%**", score_emoji, score_bar, match_score),
            false,
        )
        .field("📄 CV", cv_status, true)
        .field("📌 Statut", status_display, true);

    if let Some(tid) = thread_id {
        embed = embed.field(
            "📋 Résultats détaillés",
            format!("👉 <#{}>", tid),
            false,
        );
    }

    embed.footer(serenity::all::CreateEmbedFooter::new(format!("ID: {}", application_id)))
}

/// Exporte la fonction pour construire les boutons (utilisée par le handler)
pub fn get_status_buttons(application_id: i64, current_status: &str) -> Vec<CreateActionRow> {
    build_status_buttons(application_id, current_status)
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

async fn get_optional_attachment_content(
    interaction: &CommandInteraction,
    name: &str,
) -> Result<Option<String>, String> {
    // Get attachment ID from options
    let attachment_id = match interaction
        .data
        .options
        .iter()
        .find(|opt| opt.name == name)
        .and_then(|opt| opt.value.as_attachment_id())
    {
        Some(id) => id,
        None => return Ok(None),
    };

    // Get attachment info from resolved data
    let attachment = interaction
        .data
        .resolved
        .attachments
        .get(&attachment_id)
        .ok_or_else(|| "Fichier non trouvé".to_string())?;

    // Validate file type (only text files for job descriptions)
    let content_type = attachment.content_type.as_deref().unwrap_or("");
    let filename = &attachment.filename;

    if !content_type.contains("text/") && !filename.ends_with(".txt") && !filename.ends_with(".md") {
        return Err(format!(
            "Type de fichier non supporté: `{}`. Utilisez un fichier texte (.txt, .md).",
            content_type
        ));
    }

    // Download file
    let file_bytes = attachment
        .download()
        .await
        .map_err(|e| format!("Erreur de téléchargement: {}", e))?;

    // Convert to string
    let content = String::from_utf8(file_bytes)
        .map_err(|_| "Le fichier n'est pas un fichier texte valide (UTF-8)".to_string())?;

    if content.trim().is_empty() {
        return Err("Le fichier est vide".to_string());
    }

    Ok(Some(content))
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

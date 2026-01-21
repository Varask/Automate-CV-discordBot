use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse,
};
use std::path::PathBuf;
use tracing::{info, error, warn};
use uuid::Uuid;

use super::{CommandError, SlashCommand};
use crate::db::Database;
use crate::ClaudeClientKey;

// ============================================================================
// SendCV Command
// ============================================================================

pub struct SendCvCommand;

impl SendCvCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SendCvCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for SendCvCommand {
    fn name(&self) -> &'static str {
        "sendcv"
    }

    fn description(&self) -> &'static str {
        "Upload your CV to the bot"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description(self.description())
            .add_option(
                CreateCommandOption::new(CommandOptionType::Attachment, "cv", "Your CV file (PDF preferred)")
                    .required(true),
            )
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        // Defer immédiatement pour éviter le timeout de 3s
        interaction
            .defer(&ctx.http)
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        let user_id = interaction.user.id;
        let username = &interaction.user.name;

        // Récupérer l'attachment
        let attachment_id = interaction
            .data
            .options
            .iter()
            .find(|opt| opt.name == "cv")
            .and_then(|opt| opt.value.as_attachment_id())
            .ok_or_else(|| CommandError::MissingParameter("cv".to_string()))?;

        // Récupérer les infos de l'attachment depuis resolved
        let attachment = interaction
            .data
            .resolved
            .attachments
            .get(&attachment_id)
            .ok_or_else(|| CommandError::Internal("Attachment not found in resolved data".to_string()))?;

        // Vérifier le type de fichier
        let content_type = attachment.content_type.as_deref().unwrap_or("application/octet-stream");
        let allowed_types = ["application/pdf", "text/plain", "application/msword",
                           "application/vnd.openxmlformats-officedocument.wordprocessingml.document"];

        if !allowed_types.iter().any(|t| content_type.contains(t)) && !attachment.filename.ends_with(".pdf") {
            let response = format!(
                "❌ Type de fichier non supporté: `{}`\n\nFormats acceptés: PDF, DOC, DOCX, TXT",
                content_type
            );
            interaction
                .edit_response(&ctx.http, EditInteractionResponse::new().content(response))
                .await
                .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;
            return Ok(());
        }

        // Télécharger le fichier
        info!("Downloading CV from {} for user {}", attachment.url, user_id);
        let file_bytes = match attachment.download().await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to download attachment: {}", e);
                let response = format!("❌ Erreur lors du téléchargement: {}", e);
                interaction
                    .edit_response(&ctx.http, EditInteractionResponse::new().content(response))
                    .await
                    .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;
                return Ok(());
            }
        };

        // Créer le dossier de stockage
        let storage_dir = PathBuf::from("data/cvs");
        if let Err(e) = tokio::fs::create_dir_all(&storage_dir).await {
            error!("Failed to create storage dir: {}", e);
            return Err(CommandError::Internal(format!("Storage error: {}", e)));
        }

        // Générer un nom de fichier unique
        let extension = PathBuf::from(&attachment.filename)
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_else(|| "pdf".to_string());
        let unique_filename = format!("{}_{}.{}", user_id, Uuid::new_v4(), extension);
        let file_path = storage_dir.join(&unique_filename);

        // Sauvegarder le fichier
        if let Err(e) = tokio::fs::write(&file_path, &file_bytes).await {
            error!("Failed to write CV file: {}", e);
            return Err(CommandError::Internal(format!("File write error: {}", e)));
        }

        info!("CV saved to {:?}", file_path);

        // Sauvegarder en base de données
        let (db, claude_client) = {
            let data = ctx.data.read().await;
            let db = data.get::<Database>()
                .ok_or_else(|| CommandError::Internal("Database not found".to_string()))?
                .clone();
            let claude = data.get::<ClaudeClientKey>()
                .ok_or_else(|| CommandError::Internal("Claude client not found".to_string()))?
                .clone();
            (db, claude)
        };

        // Upsert user first
        if let Err(e) = db.upsert_user(user_id.get() as i64, username) {
            error!("Failed to upsert user: {}", e);
        }

        // Save CV metadata
        let cv_id = db.save_cv(
            user_id.get() as i64,
            &unique_filename,
            &attachment.filename,
            file_path.to_string_lossy().as_ref(),
            attachment.size as i64,
            attachment.content_type.as_deref(),
        ).map_err(|e| CommandError::Internal(format!("Database error: {}", e)))?;

        info!("CV saved to database with id {}", cv_id);

        // Mettre à jour le statut
        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content(
                    "✅ CV uploadé!\n⏳ Extraction du texte en cours..."
                ),
            )
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        // Extraire le texte du CV via Claude
        let is_pdf = extension.to_lowercase() == "pdf";
        let extracted_text = if is_pdf {
            // Encoder le PDF en base64 et demander à Claude d'extraire le texte
            let base64_content = BASE64.encode(&file_bytes);
            let prompt = format!(
                "Voici un CV au format PDF encodé en base64. Extrais et retourne UNIQUEMENT le texte brut du CV, \
                sans commentaires ni formatage. Garde la structure (sections, listes) mais en texte simple.\n\n\
                Base64 PDF (premiers 50000 caractères):\n{}",
                &base64_content[..base64_content.len().min(50000)]
            );

            match claude_client.prompt(&prompt).await {
                Ok(text) => {
                    info!("Successfully extracted {} chars from PDF", text.len());
                    Some(text)
                }
                Err(e) => {
                    warn!("Failed to extract PDF text via Claude: {}", e);
                    None
                }
            }
        } else {
            // Pour les fichiers texte, lire directement
            String::from_utf8(file_bytes.clone()).ok()
        };

        // Sauvegarder le texte extrait
        if let Some(ref text) = extracted_text {
            if let Err(e) = db.update_cv_extracted_data(cv_id, text, "{}") {
                warn!("Failed to save extracted text: {}", e);
            } else {
                info!("Extracted text saved for CV {}", cv_id);
            }
        }

        let extraction_status = if extracted_text.is_some() {
            "✅ Texte extrait avec succès"
        } else {
            "⚠️ Extraction du texte non disponible"
        };

        let response = format!(
            "✅ **CV enregistré avec succès!**\n\n\
            👤 Utilisateur: <@{}>\n\
            📄 Fichier: `{}`\n\
            📦 Taille: {} bytes\n\
            🆔 ID: `{}`\n\
            📝 {}\n\n\
            _Utilisez `/applyjob` pour postuler à une offre avec ce CV._",
            user_id,
            attachment.filename,
            attachment.size,
            cv_id,
            extraction_status
        );

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().content(response))
            .await
            .map_err(|e| CommandError::ResponseFailed(e.to_string()))?;

        Ok(())
    }
}

// ============================================================================
// DeleteCV Command
// ============================================================================

pub struct DeleteCvCommand;

impl DeleteCvCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DeleteCvCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for DeleteCvCommand {
    fn name(&self) -> &'static str {
        "deletecv"
    }

    fn description(&self) -> &'static str {
        "Delete your CV from the bot"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name()).description(self.description())
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        let user_id = interaction.user.id;

        // Récupérer la DB
        let db = {
            let data = ctx.data.read().await;
            data.get::<Database>()
                .ok_or_else(|| CommandError::Internal("Database not found".to_string()))?
                .clone()
        };

        // Vérifier s'il y a un CV actif
        let cv = db.get_active_cv(user_id.get() as i64)
            .map_err(|e| CommandError::Internal(format!("Database error: {}", e)))?;

        match cv {
            Some(cv) => {
                // Supprimer le fichier physique
                let file_path = PathBuf::from(&cv.file_path);
                if file_path.exists() {
                    if let Err(e) = std::fs::remove_file(&file_path) {
                        error!("Failed to delete CV file: {}", e);
                    } else {
                        info!("Deleted CV file: {:?}", file_path);
                    }
                }

                // Supprimer de la DB
                db.delete_active_cv(user_id.get() as i64)
                    .map_err(|e| CommandError::Internal(format!("Database error: {}", e)))?;

                let response = format!(
                    "🗑️ **CV supprimé!**\n\n📄 Fichier: `{}`",
                    cv.original_name
                );
                send_response(ctx, interaction, &response).await
            }
            None => {
                let response = "❌ Aucun CV actif trouvé.\n\nUtilisez `/sendcv` pour envoyer un CV.";
                send_response(ctx, interaction, response).await
            }
        }
    }
}

// ============================================================================
// ListMyCvs Command
// ============================================================================

pub struct ListMyCvsCommand;

impl ListMyCvsCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ListMyCvsCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlashCommand for ListMyCvsCommand {
    fn name(&self) -> &'static str {
        "listmycvs"
    }

    fn description(&self) -> &'static str {
        "List your stored CVs"
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name()).description(self.description())
    }

    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> Result<(), CommandError> {
        let user_id = interaction.user.id;

        // Récupérer la DB
        let db = {
            let data = ctx.data.read().await;
            data.get::<Database>()
                .ok_or_else(|| CommandError::Internal("Database not found".to_string()))?
                .clone()
        };

        // Récupérer la liste des CVs
        let cvs = db.list_user_cvs(user_id.get() as i64)
            .map_err(|e| CommandError::Internal(format!("Database error: {}", e)))?;

        if cvs.is_empty() {
            let response = "📋 **Vos CVs**\n\n_Aucun CV enregistré._\n\nUtilisez `/sendcv` pour envoyer un CV.";
            send_response(ctx, interaction, response).await
        } else {
            let mut response = format!("📋 **Vos CVs** ({} total)\n\n", cvs.len());

            for cv in cvs {
                let status = if cv.is_active { "✅ Actif" } else { "⬜ Inactif" };
                let size_kb = cv.file_size / 1024;
                response.push_str(&format!(
                    "{} **{}**\n  └ ID: `{}` | {} Ko | {}\n\n",
                    status,
                    cv.original_name,
                    cv.id,
                    size_kb,
                    cv.created_at.split('T').next().unwrap_or(&cv.created_at)
                ));
            }

            send_response(ctx, interaction, &response).await
        }
    }
}

// ============================================================================
// Helper
// ============================================================================

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
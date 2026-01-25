// Utilitaires pour les opérations CRUD sur la base de données
use rusqlite::{Connection, Result, params, Row, OptionalExtension};
use serde::{Deserialize, Serialize};

// ============================================================================
// MODELS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,  // Discord user ID
    pub username: String,
    pub locale: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseCv {
    pub id: i64,
    pub user_id: i64,
    pub filename: String,
    pub original_name: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: Option<String>,
    pub extracted_text: Option<String>,
    pub parsed_data: Option<String>,  // JSON string
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobApplication {
    pub id: i64,
    pub user_id: i64,
    pub base_cv_id: i64,
    pub job_title: Option<String>,
    pub company: Option<String>,
    pub location: Option<String>,
    pub job_url: Option<String>,
    pub raw_job_description: String,
    pub job_synthesis: Option<String>,
    pub required_skills: Option<String>,  // JSON
    pub matching_skills: Option<String>,  // JSON
    pub missing_skills: Option<String>,   // JSON
    pub match_score: Option<i32>,
    pub salary_min: Option<i32>,
    pub salary_max: Option<i32>,
    pub salary_currency: String,
    pub salary_analysis: Option<String>,
    pub generated_cv_path: Option<String>,
    pub generated_cv_format: String,
    pub cover_letter: Option<String>,
    pub cover_letter_generated_at: Option<String>,
    pub thread_id: Option<i64>,           // Discord thread ID
    pub status: String,
    pub applied_at: Option<String>,
    pub notes: Option<String>,
    pub reminder_date: Option<String>,
    pub reminder_sent: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: i64,
    pub user_id: i64,
    pub application_id: Option<i64>,
    pub channel_id: i64,
    pub reminder_date: String,
    pub message: String,
    pub is_sent: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationStatusHistory {
    pub id: i64,
    pub application_id: i64,
    pub old_status: Option<String>,
    pub new_status: String,
    pub note: Option<String>,
    pub changed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStats {
    pub total_applications: i32,
    pub by_status: Vec<(String, i32)>,
    pub avg_match_score: Option<f64>,
    pub top_companies: Vec<(String, i32)>,
}

// ============================================================================
// ROW MAPPERS
// ============================================================================

fn map_user(row: &Row) -> rusqlite::Result<User> {
    Ok(User {
        id: row.get(0)?,
        username: row.get(1)?,
        locale: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn map_base_cv(row: &Row) -> rusqlite::Result<BaseCv> {
    Ok(BaseCv {
        id: row.get(0)?,
        user_id: row.get(1)?,
        filename: row.get(2)?,
        original_name: row.get(3)?,
        file_path: row.get(4)?,
        file_size: row.get(5)?,
        mime_type: row.get(6)?,
        extracted_text: row.get(7)?,
        parsed_data: row.get(8)?,
        is_active: row.get::<_, i32>(9)? == 1,
        created_at: row.get(10)?,
    })
}

fn map_job_application(row: &Row) -> rusqlite::Result<JobApplication> {
    Ok(JobApplication {
        id: row.get(0)?,
        user_id: row.get(1)?,
        base_cv_id: row.get(2)?,
        job_title: row.get(3)?,
        company: row.get(4)?,
        location: row.get(5)?,
        job_url: row.get(6)?,
        raw_job_description: row.get(7)?,
        job_synthesis: row.get(8)?,
        required_skills: row.get(9)?,
        matching_skills: row.get(10)?,
        missing_skills: row.get(11)?,
        match_score: row.get(12)?,
        salary_min: row.get(13)?,
        salary_max: row.get(14)?,
        salary_currency: row.get(15)?,
        salary_analysis: row.get(16)?,
        generated_cv_path: row.get(17)?,
        generated_cv_format: row.get(18)?,
        cover_letter: row.get(19)?,
        cover_letter_generated_at: row.get(20)?,
        thread_id: row.get(21)?,
        status: row.get(22)?,
        applied_at: row.get(23)?,
        notes: row.get(24)?,
        reminder_date: row.get(25)?,
        reminder_sent: row.get::<_, i32>(26)? == 1,
        created_at: row.get(27)?,
        updated_at: row.get(28)?,
    })
}

fn map_reminder(row: &Row) -> rusqlite::Result<Reminder> {
    Ok(Reminder {
        id: row.get(0)?,
        user_id: row.get(1)?,
        application_id: row.get(2)?,
        channel_id: row.get(3)?,
        reminder_date: row.get(4)?,
        message: row.get(5)?,
        is_sent: row.get::<_, i32>(6)? == 1,
        created_at: row.get(7)?,
    })
}

// ============================================================================
// USER OPERATIONS
// ============================================================================

/// Crée ou met à jour un utilisateur (upsert)
pub fn upsert_user(conn: &Connection, user_id: i64, username: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO users (id, username, updated_at) 
         VALUES (?1, ?2, CURRENT_TIMESTAMP)
         ON CONFLICT(id) DO UPDATE SET 
            username = excluded.username,
            updated_at = CURRENT_TIMESTAMP",
        params![user_id, username],
    )?;
    Ok(())
}

/// Récupère un utilisateur par son ID Discord
pub fn get_user(conn: &Connection, user_id: i64) -> Result<Option<User>> {
    let mut stmt = conn.prepare(
        "SELECT id, username, locale, created_at, updated_at FROM users WHERE id = ?1"
    )?;
    
    let user = stmt.query_row(params![user_id], map_user).optional()?;
    Ok(user)
}

// ============================================================================
// CV OPERATIONS
// ============================================================================

/// Sauvegarde un nouveau CV et le marque comme actif (désactive les précédents)
pub fn save_cv(
    conn: &Connection,
    user_id: i64,
    filename: &str,
    original_name: &str,
    file_path: &str,
    file_size: i64,
    mime_type: Option<&str>,
) -> Result<i64> {
    // Désactiver les anciens CVs de l'utilisateur
    conn.execute(
        "UPDATE base_cvs SET is_active = 0 WHERE user_id = ?1",
        params![user_id],
    )?;

    // Insérer le nouveau CV
    conn.execute(
        "INSERT INTO base_cvs (user_id, filename, original_name, file_path, file_size, mime_type, is_active)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
        params![user_id, filename, original_name, file_path, file_size, mime_type],
    )?;

    Ok(conn.last_insert_rowid())
}

/// Récupère le CV actif d'un utilisateur
pub fn get_active_cv(conn: &Connection, user_id: i64) -> Result<Option<BaseCv>> {
    let mut stmt = conn.prepare(
        "SELECT id, user_id, filename, original_name, file_path, file_size, 
                mime_type, extracted_text, parsed_data, is_active, created_at
         FROM base_cvs 
         WHERE user_id = ?1 AND is_active = 1"
    )?;

    let cv = stmt.query_row(params![user_id], map_base_cv).optional()?;
    Ok(cv)
}

/// Liste tous les CVs d'un utilisateur
pub fn list_user_cvs(conn: &Connection, user_id: i64) -> Result<Vec<BaseCv>> {
    let mut stmt = conn.prepare(
        "SELECT id, user_id, filename, original_name, file_path, file_size,
                mime_type, extracted_text, parsed_data, is_active, created_at
         FROM base_cvs
         WHERE user_id = ?1
         ORDER BY created_at DESC"
    )?;

    let cvs = stmt
        .query_map(params![user_id], map_base_cv)?
        .filter_map(|r| r.ok())
        .collect();

    Ok(cvs)
}

/// Supprime le CV actif d'un utilisateur
pub fn delete_active_cv(conn: &Connection, user_id: i64) -> Result<bool> {
    let rows = conn.execute(
        "DELETE FROM base_cvs WHERE user_id = ?1 AND is_active = 1",
        params![user_id],
    )?;
    Ok(rows > 0)
}

/// Met à jour les données extraites d'un CV
pub fn update_cv_extracted_data(
    conn: &Connection,
    cv_id: i64,
    extracted_text: &str,
    parsed_data: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE base_cvs SET extracted_text = ?1, parsed_data = ?2 WHERE id = ?3",
        params![extracted_text, parsed_data, cv_id],
    )?;
    Ok(())
}

// ============================================================================
// JOB APPLICATION OPERATIONS
// ============================================================================

/// Crée une nouvelle candidature
pub fn create_application(
    conn: &Connection,
    user_id: i64,
    base_cv_id: i64,
    job_title: Option<&str>,
    company: Option<&str>,
    location: Option<&str>,
    job_url: Option<&str>,
    raw_job_description: &str,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO job_applications 
         (user_id, base_cv_id, job_title, company, location, job_url, raw_job_description)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![user_id, base_cv_id, job_title, company, location, job_url, raw_job_description],
    )?;

    Ok(conn.last_insert_rowid())
}

/// Met à jour le thread_id d'une candidature
pub fn update_application_thread(
    conn: &Connection,
    application_id: i64,
    thread_id: i64,
) -> Result<()> {
    conn.execute(
        "UPDATE job_applications SET thread_id = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![thread_id, application_id],
    )?;
    Ok(())
}

/// Met à jour une candidature avec les résultats de l'analyse AI
pub fn update_application_analysis(
    conn: &Connection,
    application_id: i64,
    job_synthesis: &str,
    required_skills: &str,
    matching_skills: &str,
    missing_skills: &str,
    match_score: i32,
) -> Result<()> {
    conn.execute(
        "UPDATE job_applications SET
            job_synthesis = ?1,
            required_skills = ?2,
            matching_skills = ?3,
            missing_skills = ?4,
            match_score = ?5,
            updated_at = CURRENT_TIMESTAMP
         WHERE id = ?6",
        params![job_synthesis, required_skills, matching_skills, missing_skills, match_score, application_id],
    )?;
    Ok(())
}

/// Met à jour une candidature avec l'analyse salariale
pub fn update_application_salary(
    conn: &Connection,
    application_id: i64,
    salary_min: Option<i32>,
    salary_max: Option<i32>,
    salary_analysis: &str,
    market_salary_low: Option<i32>,
    market_salary_mid: Option<i32>,
    market_salary_high: Option<i32>,
) -> Result<()> {
    conn.execute(
        "UPDATE job_applications SET
            salary_min = ?1,
            salary_max = ?2,
            salary_analysis = ?3,
            market_salary_low = ?4,
            market_salary_mid = ?5,
            market_salary_high = ?6,
            updated_at = CURRENT_TIMESTAMP
         WHERE id = ?7",
        params![salary_min, salary_max, salary_analysis, market_salary_low, market_salary_mid, market_salary_high, application_id],
    )?;
    Ok(())
}

/// Met à jour le chemin du CV généré
pub fn update_application_generated_cv(
    conn: &Connection,
    application_id: i64,
    generated_cv_path: &str,
    format: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE job_applications SET
            generated_cv_path = ?1,
            generated_cv_format = ?2,
            updated_at = CURRENT_TIMESTAMP
         WHERE id = ?3",
        params![generated_cv_path, format, application_id],
    )?;
    Ok(())
}

/// Récupère une candidature par son ID
pub fn get_application(conn: &Connection, application_id: i64) -> Result<Option<JobApplication>> {
    let mut stmt = conn.prepare(
        "SELECT id, user_id, base_cv_id, job_title, company, location, job_url,
                raw_job_description, job_synthesis, required_skills, matching_skills,
                missing_skills, match_score, salary_min, salary_max, salary_currency,
                salary_analysis, generated_cv_path, generated_cv_format,
                cover_letter, cover_letter_generated_at, thread_id,
                status, applied_at, notes, reminder_date, reminder_sent,
                created_at, updated_at
         FROM job_applications WHERE id = ?1"
    )?;

    let app = stmt.query_row(params![application_id], map_job_application).optional()?;
    Ok(app)
}

const JOB_APPLICATION_SELECT: &str = "SELECT id, user_id, base_cv_id, job_title, company, location, job_url,
        raw_job_description, job_synthesis, required_skills, matching_skills,
        missing_skills, match_score, salary_min, salary_max, salary_currency,
        salary_analysis, generated_cv_path, generated_cv_format,
        cover_letter, cover_letter_generated_at, thread_id,
        status, applied_at, notes, reminder_date, reminder_sent,
        created_at, updated_at
 FROM job_applications";

/// Liste les candidatures d'un utilisateur avec filtres
pub fn list_applications(
    conn: &Connection,
    user_id: i64,
    status_filter: Option<&str>,
    limit: i64,
) -> Result<Vec<JobApplication>> {
    match status_filter {
        Some(status) => {
            let sql = format!(
                "{} WHERE user_id = ?1 AND status = ?2 ORDER BY created_at DESC LIMIT ?3",
                JOB_APPLICATION_SELECT
            );
            let mut stmt = conn.prepare(&sql)?;
            let apps: Vec<JobApplication> = stmt
                .query_map(params![user_id, status, limit], map_job_application)?
                .filter_map(|r| r.ok())
                .collect();
            Ok(apps)
        }
        None => {
            let sql = format!(
                "{} WHERE user_id = ?1 ORDER BY created_at DESC LIMIT ?2",
                JOB_APPLICATION_SELECT
            );
            let mut stmt = conn.prepare(&sql)?;
            let apps: Vec<JobApplication> = stmt
                .query_map(params![user_id, limit], map_job_application)?
                .filter_map(|r| r.ok())
                .collect();
            Ok(apps)
        }
    }
}

/// Met à jour le statut d'une candidature
pub fn update_application_status(
    conn: &Connection,
    application_id: i64,
    user_id: i64,
    new_status: &str,
    note: Option<&str>,
) -> Result<bool> {
    // Récupérer l'ancien statut
    let mut stmt = conn.prepare(
        "SELECT status FROM job_applications WHERE id = ?1 AND user_id = ?2"
    )?;
    let old_status: Option<String> = stmt
        .query_row(params![application_id, user_id], |row: &Row| row.get(0))
        .optional()?;

    if old_status.is_none() {
        return Ok(false);  // Application non trouvée ou pas à cet utilisateur
    }

    // Mettre à jour le statut
    let applied_at_update = if new_status == "applied" {
        ", applied_at = CURRENT_TIMESTAMP"
    } else {
        ""
    };

    conn.execute(
        &format!(
            "UPDATE job_applications SET status = ?1, updated_at = CURRENT_TIMESTAMP{} WHERE id = ?2",
            applied_at_update
        ),
        params![new_status, application_id],
    )?;

    // Ajouter à l'historique
    conn.execute(
        "INSERT INTO application_status_history (application_id, old_status, new_status, note)
         VALUES (?1, ?2, ?3, ?4)",
        params![application_id, old_status, new_status, note],
    )?;

    Ok(true)
}

// ============================================================================
// STATISTICS
// ============================================================================

/// Récupère les statistiques d'un utilisateur
pub fn get_user_stats(conn: &Connection, user_id: i64) -> Result<UserStats> {
    // Total applications
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM job_applications WHERE user_id = ?1")?;
    let total: i32 = stmt.query_row(params![user_id], |row: &Row| row.get(0))?;

    // By status
    let mut stmt = conn.prepare(
        "SELECT status, COUNT(*) FROM job_applications WHERE user_id = ?1 GROUP BY status"
    )?;
    let by_status: Vec<(String, i32)> = stmt
        .query_map(params![user_id], |row: &Row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    // Average match score
    let mut stmt = conn.prepare(
        "SELECT AVG(match_score) FROM job_applications WHERE user_id = ?1 AND match_score IS NOT NULL"
    )?;
    let avg_score: Option<f64> = stmt
        .query_row(params![user_id], |row: &Row| row.get(0))
        .optional()?
        .flatten();

    // Top companies
    let mut stmt = conn.prepare(
        "SELECT company, COUNT(*) as cnt FROM job_applications 
         WHERE user_id = ?1 AND company IS NOT NULL 
         GROUP BY company ORDER BY cnt DESC LIMIT 5"
    )?;
    let top_companies: Vec<(String, i32)> = stmt
        .query_map(params![user_id], |row: &Row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(UserStats {
        total_applications: total,
        by_status,
        avg_match_score: avg_score,
        top_companies,
    })
}

// ============================================================================
// ADMIN OPERATIONS
// ============================================================================

/// Liste tous les CVs (admin)
pub fn list_all_cvs(conn: &Connection) -> Result<Vec<(i64, String, BaseCv)>> {
    let mut stmt = conn.prepare(
        "SELECT u.id, u.username, c.id, c.user_id, c.filename, c.original_name, 
                c.file_path, c.file_size, c.mime_type, c.extracted_text, 
                c.parsed_data, c.is_active, c.created_at
         FROM base_cvs c
         JOIN users u ON c.user_id = u.id
         WHERE c.is_active = 1
         ORDER BY c.created_at DESC"
    )?;

    let results = stmt
        .query_map([], |row: &Row| {
            let user_id: i64 = row.get(0)?;
            let username: String = row.get(1)?;
            let cv = BaseCv {
                id: row.get(2)?,
                user_id: row.get(3)?,
                filename: row.get(4)?,
                original_name: row.get(5)?,
                file_path: row.get(6)?,
                file_size: row.get(7)?,
                mime_type: row.get(8)?,
                extracted_text: row.get(9)?,
                parsed_data: row.get(10)?,
                is_active: row.get::<_, i32>(11)? == 1,
                created_at: row.get(12)?,
            };
            Ok((user_id, username, cv))
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(results)
}

/// Supprime tous les CVs (admin)
pub fn clear_all_cvs(conn: &Connection) -> Result<usize> {
    let count = conn.execute("DELETE FROM base_cvs", [])?;
    Ok(count)
}

// ============================================================================
// COVER LETTER OPERATIONS
// ============================================================================

/// Sauvegarde une lettre de motivation pour une candidature
pub fn save_cover_letter(
    conn: &Connection,
    application_id: i64,
    cover_letter: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE job_applications SET
            cover_letter = ?1,
            cover_letter_generated_at = CURRENT_TIMESTAMP,
            updated_at = CURRENT_TIMESTAMP
         WHERE id = ?2",
        params![cover_letter, application_id],
    )?;
    Ok(())
}

/// Récupère la lettre de motivation d'une candidature
pub fn get_cover_letter(conn: &Connection, application_id: i64) -> Result<Option<String>> {
    let mut stmt = conn.prepare(
        "SELECT cover_letter FROM job_applications WHERE id = ?1"
    )?;
    let letter: Option<String> = stmt
        .query_row(params![application_id], |row| row.get(0))
        .optional()?
        .flatten();
    Ok(letter)
}

/// Liste les candidatures avec lettre de motivation pour un utilisateur
pub fn list_applications_with_cover_letters(
    conn: &Connection,
    user_id: i64,
    limit: i64,
) -> Result<Vec<JobApplication>> {
    let sql = format!(
        "{} WHERE user_id = ?1 AND cover_letter IS NOT NULL ORDER BY cover_letter_generated_at DESC LIMIT ?2",
        JOB_APPLICATION_SELECT
    );
    let mut stmt = conn.prepare(&sql)?;
    let apps: Vec<JobApplication> = stmt
        .query_map(params![user_id, limit], map_job_application)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(apps)
}

// ============================================================================
// REMINDER OPERATIONS
// ============================================================================

/// Crée un rappel pour une candidature
pub fn set_application_reminder(
    conn: &Connection,
    application_id: i64,
    reminder_date: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE job_applications SET
            reminder_date = ?1,
            reminder_sent = 0,
            updated_at = CURRENT_TIMESTAMP
         WHERE id = ?2",
        params![reminder_date, application_id],
    )?;
    Ok(())
}

/// Supprime un rappel de candidature
pub fn clear_application_reminder(conn: &Connection, application_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE job_applications SET
            reminder_date = NULL,
            reminder_sent = 0,
            updated_at = CURRENT_TIMESTAMP
         WHERE id = ?1",
        params![application_id],
    )?;
    Ok(())
}

/// Marque un rappel de candidature comme envoyé
pub fn mark_application_reminder_sent(conn: &Connection, application_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE job_applications SET
            reminder_sent = 1,
            updated_at = CURRENT_TIMESTAMP
         WHERE id = ?1",
        params![application_id],
    )?;
    Ok(())
}

/// Liste les rappels de candidatures en attente (date passée et non envoyés)
pub fn get_pending_application_reminders(conn: &Connection) -> Result<Vec<JobApplication>> {
    let sql = format!(
        "{} WHERE reminder_date IS NOT NULL
         AND reminder_sent = 0
         AND datetime(reminder_date) <= datetime('now')
         ORDER BY reminder_date ASC",
        JOB_APPLICATION_SELECT
    );
    let mut stmt = conn.prepare(&sql)?;
    let apps: Vec<JobApplication> = stmt
        .query_map([], map_job_application)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(apps)
}

/// Liste les rappels à venir pour un utilisateur
pub fn list_user_application_reminders(
    conn: &Connection,
    user_id: i64,
) -> Result<Vec<JobApplication>> {
    let sql = format!(
        "{} WHERE user_id = ?1
         AND reminder_date IS NOT NULL
         AND reminder_sent = 0
         ORDER BY reminder_date ASC",
        JOB_APPLICATION_SELECT
    );
    let mut stmt = conn.prepare(&sql)?;
    let apps: Vec<JobApplication> = stmt
        .query_map(params![user_id], map_job_application)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(apps)
}

// ============================================================================
// STANDALONE REMINDER OPERATIONS
// ============================================================================

/// Crée un rappel standalone (non lié à une candidature)
pub fn create_reminder(
    conn: &Connection,
    user_id: i64,
    application_id: Option<i64>,
    channel_id: i64,
    reminder_date: &str,
    message: &str,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO reminders (user_id, application_id, channel_id, reminder_date, message)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![user_id, application_id, channel_id, reminder_date, message],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Récupère un rappel par son ID
pub fn get_reminder(conn: &Connection, reminder_id: i64) -> Result<Option<Reminder>> {
    let mut stmt = conn.prepare(
        "SELECT id, user_id, application_id, channel_id, reminder_date, message, is_sent, created_at
         FROM reminders WHERE id = ?1"
    )?;
    let reminder = stmt.query_row(params![reminder_id], map_reminder).optional()?;
    Ok(reminder)
}

/// Liste les rappels d'un utilisateur
pub fn list_user_reminders(conn: &Connection, user_id: i64) -> Result<Vec<Reminder>> {
    let mut stmt = conn.prepare(
        "SELECT id, user_id, application_id, channel_id, reminder_date, message, is_sent, created_at
         FROM reminders WHERE user_id = ?1 AND is_sent = 0
         ORDER BY reminder_date ASC"
    )?;
    let reminders: Vec<Reminder> = stmt
        .query_map(params![user_id], map_reminder)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(reminders)
}

/// Supprime un rappel
pub fn delete_reminder(conn: &Connection, reminder_id: i64, user_id: i64) -> Result<bool> {
    let rows = conn.execute(
        "DELETE FROM reminders WHERE id = ?1 AND user_id = ?2",
        params![reminder_id, user_id],
    )?;
    Ok(rows > 0)
}

/// Marque un rappel comme envoyé
pub fn mark_reminder_sent(conn: &Connection, reminder_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE reminders SET is_sent = 1 WHERE id = ?1",
        params![reminder_id],
    )?;
    Ok(())
}

/// Liste tous les rappels en attente (date passée et non envoyés)
pub fn get_pending_reminders(conn: &Connection) -> Result<Vec<Reminder>> {
    let mut stmt = conn.prepare(
        "SELECT id, user_id, application_id, channel_id, reminder_date, message, is_sent, created_at
         FROM reminders
         WHERE is_sent = 0 AND datetime(reminder_date) <= datetime('now')
         ORDER BY reminder_date ASC"
    )?;
    let reminders: Vec<Reminder> = stmt
        .query_map([], map_reminder)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(reminders)
}
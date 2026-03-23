// Le but de ce fichier est d'initialiser la base de données
// Créer la base de données si elle n'existe pas
use rusqlite::{Connection, Result};
use std::path::Path;
use std::fs;
use tracing::info;

const DB_DIR: &str = "dbLookout";
const DB_NAME: &str = "bot.db";

/// Retourne le chemin complet vers la base de données
pub fn get_db_path() -> String {
    format!("{}/{}", DB_DIR, DB_NAME)
}

/// Initialise la base de données et crée les tables si nécessaire
pub fn init_database() -> Result<Connection> {
    // Créer le dossier dbLookout s'il n'existe pas
    if !Path::new(DB_DIR).exists() {
        fs::create_dir_all(DB_DIR).expect("Failed to create database directory");
        info!("Created database directory: {}", DB_DIR);
    }

    let db_path = get_db_path();
    let conn = Connection::open(&db_path)?;
    
    info!("Connected to database: {}", db_path);

    // Activer les foreign keys
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    // Créer les tables
    create_tables(&conn)?;

    info!("Database initialized successfully");
    Ok(conn)
}

/// Crée toutes les tables de la base de données
fn create_tables(conn: &Connection) -> Result<()> {
    // Table: users
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id              INTEGER PRIMARY KEY,  -- Discord user ID
            username        TEXT NOT NULL,
            locale          TEXT DEFAULT 'fr',
            created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    info!("Table 'users' ready");

    // Table: base_cvs
    conn.execute(
        "CREATE TABLE IF NOT EXISTS base_cvs (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id         INTEGER NOT NULL,
            filename        TEXT NOT NULL,
            original_name   TEXT NOT NULL,
            file_path       TEXT NOT NULL,
            file_size       INTEGER NOT NULL,
            mime_type       TEXT,
            extracted_text  TEXT,
            parsed_data     TEXT,  -- JSON
            is_active       INTEGER DEFAULT 1,
            created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        [],
    )?;
    info!("Table 'base_cvs' ready");

    // Table: job_applications
    conn.execute(
        "CREATE TABLE IF NOT EXISTS job_applications (
            id                      INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id                 INTEGER NOT NULL,
            base_cv_id              INTEGER,  -- nullable: candidature possible sans CV uploadé
            
            -- Job offer info
            job_title               TEXT,
            company                 TEXT,
            location                TEXT,
            job_url                 TEXT,
            raw_job_description     TEXT NOT NULL,
            
            -- Generated outputs
            job_synthesis           TEXT,
            required_skills         TEXT,  -- JSON array
            matching_skills         TEXT,  -- JSON array
            missing_skills          TEXT,  -- JSON array
            match_score             INTEGER,
            
            -- Salary analysis
            salary_min              INTEGER,
            salary_max              INTEGER,
            salary_currency         TEXT DEFAULT 'EUR',
            salary_analysis         TEXT,
            market_salary_low       INTEGER,
            market_salary_mid       INTEGER,
            market_salary_high      INTEGER,
            
            -- Generated CV
            generated_cv_path       TEXT,
            generated_cv_format     TEXT DEFAULT 'pdf',

            -- Cover letter
            cover_letter            TEXT,
            cover_letter_generated_at DATETIME,

            -- Discord tracking
            thread_id               INTEGER,  -- Discord thread ID for detailed results

            -- Application tracking
            status                  TEXT DEFAULT 'generated',
            applied_at              DATETIME,
            notes                   TEXT,

            -- Reminder
            reminder_date           DATETIME,
            reminder_sent           INTEGER DEFAULT 0,
            
            created_at              DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at              DATETIME DEFAULT CURRENT_TIMESTAMP,
            
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY (base_cv_id) REFERENCES base_cvs(id)
        )",
        [],
    )?;
    info!("Table 'job_applications' ready");

    // Table: application_status_history
    conn.execute(
        "CREATE TABLE IF NOT EXISTS application_status_history (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            application_id  INTEGER NOT NULL,
            old_status      TEXT,
            new_status      TEXT NOT NULL,
            note            TEXT,
            changed_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (application_id) REFERENCES job_applications(id) ON DELETE CASCADE
        )",
        [],
    )?;
    info!("Table 'application_status_history' ready");

    // Table: reminders (standalone reminders not linked to applications)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS reminders (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id         INTEGER NOT NULL,
            application_id  INTEGER,  -- Optional link to application
            channel_id      INTEGER NOT NULL,  -- Discord channel for notification
            reminder_date   DATETIME NOT NULL,
            message         TEXT NOT NULL,
            is_sent         INTEGER DEFAULT 0,
            created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY (application_id) REFERENCES job_applications(id) ON DELETE SET NULL
        )",
        [],
    )?;
    info!("Table 'reminders' ready");

    // Exécuter les migrations pour les colonnes manquantes
    run_migrations(conn)?;

    // Créer les index pour les performances
    create_indexes(conn)?;

    Ok(())
}

/// Exécute les migrations versionnées
fn run_migrations(conn: &Connection) -> Result<()> {
    // Créer la table de version de migrations si elle n'existe pas
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version  INTEGER PRIMARY KEY,
            applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Helper: vérifier si une migration a déjà été appliquée
    let is_applied = |version: i64| -> Result<bool> {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
                [version],
                |row| row.get(0),
            )?;
        Ok(count > 0)
    };

    // Migration 1: Ajouter reminder_date et reminder_sent à job_applications
    if !is_applied(1)? {
        let _ = conn.execute("ALTER TABLE job_applications ADD COLUMN reminder_date DATETIME", []);
        let _ = conn.execute("ALTER TABLE job_applications ADD COLUMN reminder_sent INTEGER DEFAULT 0", []);
        conn.execute("INSERT INTO schema_migrations (version) VALUES (1)", [])?;
    }

    // Migration 2: Rendre base_cv_id nullable (reconstruit la table)
    if !is_applied(2)? {
        // Vérifier si la colonne est NOT NULL en inspectant le schéma
        let schema: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='job_applications'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_default();

        // Recréer uniquement si base_cv_id NOT NULL est encore présent
        if schema.to_uppercase().contains("BASE_CV_ID") && schema.to_uppercase().contains("NOT NULL") {
            conn.execute_batch(
                "BEGIN;
                CREATE TABLE IF NOT EXISTS job_applications_migration AS SELECT * FROM job_applications;
                DROP TABLE job_applications;
                CREATE TABLE job_applications (
                    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
                    user_id                 INTEGER NOT NULL,
                    base_cv_id              INTEGER,
                    job_title               TEXT,
                    company                 TEXT,
                    location                TEXT,
                    job_url                 TEXT,
                    raw_job_description     TEXT NOT NULL,
                    job_synthesis           TEXT,
                    required_skills         TEXT,
                    matching_skills         TEXT,
                    missing_skills          TEXT,
                    match_score             INTEGER,
                    salary_min              INTEGER,
                    salary_max              INTEGER,
                    salary_currency         TEXT DEFAULT 'EUR',
                    salary_analysis         TEXT,
                    market_salary_low       INTEGER,
                    market_salary_mid       INTEGER,
                    market_salary_high      INTEGER,
                    generated_cv_path       TEXT,
                    generated_cv_format     TEXT DEFAULT 'pdf',
                    cover_letter            TEXT,
                    cover_letter_generated_at DATETIME,
                    thread_id               INTEGER,
                    status                  TEXT DEFAULT 'generated',
                    applied_at              DATETIME,
                    notes                   TEXT,
                    reminder_date           DATETIME,
                    reminder_sent           INTEGER DEFAULT 0,
                    created_at              DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at              DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                    FOREIGN KEY (base_cv_id) REFERENCES base_cvs(id)
                );
                INSERT INTO job_applications SELECT * FROM job_applications_migration;
                DROP TABLE job_applications_migration;
                COMMIT;",
            )?;
        }
        conn.execute("INSERT INTO schema_migrations (version) VALUES (2)", [])?;
    }

    Ok(())
}

/// Crée les tables pour les tests (version publique pour les tests uniquement)
#[cfg(test)]
pub fn create_tables_for_test(conn: &Connection) -> Result<()> {
    create_tables(conn)
}

/// Crée les index pour optimiser les requêtes
fn create_indexes(conn: &Connection) -> Result<()> {
    let indexes = [
        "CREATE INDEX IF NOT EXISTS idx_base_cvs_user ON base_cvs(user_id)",
        "CREATE INDEX IF NOT EXISTS idx_base_cvs_active ON base_cvs(user_id, is_active)",
        "CREATE INDEX IF NOT EXISTS idx_job_applications_user ON job_applications(user_id)",
        "CREATE INDEX IF NOT EXISTS idx_job_applications_status ON job_applications(status)",
        "CREATE INDEX IF NOT EXISTS idx_job_applications_user_status ON job_applications(user_id, status)",
        "CREATE INDEX IF NOT EXISTS idx_job_applications_reminder ON job_applications(reminder_date, reminder_sent)",
        "CREATE INDEX IF NOT EXISTS idx_status_history_app ON application_status_history(application_id)",
        "CREATE INDEX IF NOT EXISTS idx_reminders_user ON reminders(user_id)",
        "CREATE INDEX IF NOT EXISTS idx_reminders_pending ON reminders(reminder_date, is_sent)",
    ];

    for idx in indexes {
        conn.execute(idx, [])?;
    }
    info!("Indexes created");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_database() {
        // Test avec une DB en mémoire
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        create_tables(&conn).unwrap();
        
        // Vérifier que les tables existent
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        
        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"base_cvs".to_string()));
        assert!(tables.contains(&"job_applications".to_string()));
        assert!(tables.contains(&"application_status_history".to_string()));
    }
}
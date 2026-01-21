// Le but de ce fichier est d'initialiser la base de données
// Créer la base de données si elle n'existe pas
use rusqlite::{Connection, Result};
use std::path::Path;
use std::fs;

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
        println!("📁 Created database directory: {}", DB_DIR);
    }

    let db_path = get_db_path();
    let conn = Connection::open(&db_path)?;
    
    println!("🗄️  Connected to database: {}", db_path);

    // Activer les foreign keys
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    // Créer les tables
    create_tables(&conn)?;

    println!("✅ Database initialized successfully");
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
    println!("  📋 Table 'users' ready");

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
    println!("  📋 Table 'base_cvs' ready");

    // Table: job_applications
    conn.execute(
        "CREATE TABLE IF NOT EXISTS job_applications (
            id                      INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id                 INTEGER NOT NULL,
            base_cv_id              INTEGER NOT NULL,
            
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
            
            -- Application tracking
            status                  TEXT DEFAULT 'generated',
            applied_at              DATETIME,
            notes                   TEXT,
            
            created_at              DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at              DATETIME DEFAULT CURRENT_TIMESTAMP,
            
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY (base_cv_id) REFERENCES base_cvs(id)
        )",
        [],
    )?;
    println!("  📋 Table 'job_applications' ready");

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
    println!("  📋 Table 'application_status_history' ready");

    // Créer les index pour les performances
    create_indexes(conn)?;

    Ok(())
}

/// Crée les index pour optimiser les requêtes
fn create_indexes(conn: &Connection) -> Result<()> {
    let indexes = [
        "CREATE INDEX IF NOT EXISTS idx_base_cvs_user ON base_cvs(user_id)",
        "CREATE INDEX IF NOT EXISTS idx_base_cvs_active ON base_cvs(user_id, is_active)",
        "CREATE INDEX IF NOT EXISTS idx_job_applications_user ON job_applications(user_id)",
        "CREATE INDEX IF NOT EXISTS idx_job_applications_status ON job_applications(status)",
        "CREATE INDEX IF NOT EXISTS idx_job_applications_user_status ON job_applications(user_id, status)",
        "CREATE INDEX IF NOT EXISTS idx_status_history_app ON application_status_history(application_id)",
    ];

    for idx in indexes {
        conn.execute(idx, [])?;
    }
    println!("  🔍 Indexes created");

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
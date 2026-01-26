// Module de base de données
// Gestion SQLite pour le bot Discord CV

pub mod init;
pub mod utilities;

pub use init::init_database;
pub use utilities::*;

use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Wrapper thread-safe pour la connexion SQLite
/// Nécessaire car rusqlite::Connection n'est pas Sync
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Crée une nouvelle instance avec initialisation de la DB
    pub fn new() -> Result<Self, rusqlite::Error> {
        let conn = init_database()?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Crée une instance en mémoire (pour les tests)
    #[cfg(test)]
    pub fn in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        
        // Créer les tables manuellement pour les tests
        init::create_tables_for_test(&conn)?;
        
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Exécute une opération avec la connexion
    pub fn with_conn<F, T>(&self, f: F) -> Result<T, rusqlite::Error>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let conn = self.conn.lock().expect("Database mutex poisoned");
        f(&conn)
    }

    // ========================================================================
    // USER METHODS
    // ========================================================================

    pub fn upsert_user(&self, user_id: i64, username: &str) -> Result<(), rusqlite::Error> {
        self.with_conn(|conn| utilities::upsert_user(conn, user_id, username))
    }

    pub fn get_user(&self, user_id: i64) -> Result<Option<User>, rusqlite::Error> {
        self.with_conn(|conn| utilities::get_user(conn, user_id))
    }

    // ========================================================================
    // CV METHODS
    // ========================================================================

    pub fn save_cv(
        &self,
        user_id: i64,
        filename: &str,
        original_name: &str,
        file_path: &str,
        file_size: i64,
        mime_type: Option<&str>,
    ) -> Result<i64, rusqlite::Error> {
        self.with_conn(|conn| {
            utilities::save_cv(conn, user_id, filename, original_name, file_path, file_size, mime_type)
        })
    }

    pub fn get_active_cv(&self, user_id: i64) -> Result<Option<BaseCv>, rusqlite::Error> {
        self.with_conn(|conn| utilities::get_active_cv(conn, user_id))
    }

    pub fn list_user_cvs(&self, user_id: i64) -> Result<Vec<BaseCv>, rusqlite::Error> {
        self.with_conn(|conn| utilities::list_user_cvs(conn, user_id))
    }

    pub fn delete_active_cv(&self, user_id: i64) -> Result<bool, rusqlite::Error> {
        self.with_conn(|conn| utilities::delete_active_cv(conn, user_id))
    }

    pub fn update_cv_extracted_data(
        &self,
        cv_id: i64,
        extracted_text: &str,
        parsed_data: &str,
    ) -> Result<(), rusqlite::Error> {
        self.with_conn(|conn| utilities::update_cv_extracted_data(conn, cv_id, extracted_text, parsed_data))
    }

    // ========================================================================
    // APPLICATION METHODS
    // ========================================================================

    pub fn create_application(
        &self,
        user_id: i64,
        base_cv_id: i64,
        job_title: Option<&str>,
        company: Option<&str>,
        location: Option<&str>,
        job_url: Option<&str>,
        raw_job_description: &str,
    ) -> Result<i64, rusqlite::Error> {
        self.with_conn(|conn| {
            utilities::create_application(
                conn, user_id, base_cv_id, job_title, company, location, job_url, raw_job_description
            )
        })
    }

    pub fn get_application(&self, application_id: i64) -> Result<Option<JobApplication>, rusqlite::Error> {
        self.with_conn(|conn| utilities::get_application(conn, application_id))
    }

    pub fn list_applications(
        &self,
        user_id: i64,
        status_filter: Option<&str>,
        limit: i64,
    ) -> Result<Vec<JobApplication>, rusqlite::Error> {
        self.with_conn(|conn| utilities::list_applications(conn, user_id, status_filter, limit))
    }

    pub fn update_application_status(
        &self,
        application_id: i64,
        user_id: i64,
        new_status: &str,
        note: Option<&str>,
    ) -> Result<bool, rusqlite::Error> {
        self.with_conn(|conn| {
            utilities::update_application_status(conn, application_id, user_id, new_status, note)
        })
    }

    pub fn update_application_thread(
        &self,
        application_id: i64,
        thread_id: i64,
    ) -> Result<(), rusqlite::Error> {
        self.with_conn(|conn| utilities::update_application_thread(conn, application_id, thread_id))
    }

    pub fn update_application_analysis(
        &self,
        application_id: i64,
        job_synthesis: &str,
        required_skills: &str,
        matching_skills: &str,
        missing_skills: &str,
        match_score: i32,
    ) -> Result<(), rusqlite::Error> {
        self.with_conn(|conn| {
            utilities::update_application_analysis(
                conn, application_id, job_synthesis, required_skills, matching_skills, missing_skills, match_score
            )
        })
    }

    pub fn update_application_salary(
        &self,
        application_id: i64,
        salary_min: Option<i32>,
        salary_max: Option<i32>,
        salary_analysis: &str,
        market_salary_low: Option<i32>,
        market_salary_mid: Option<i32>,
        market_salary_high: Option<i32>,
    ) -> Result<(), rusqlite::Error> {
        self.with_conn(|conn| {
            utilities::update_application_salary(
                conn, application_id, salary_min, salary_max, salary_analysis,
                market_salary_low, market_salary_mid, market_salary_high
            )
        })
    }

    pub fn update_application_generated_cv(
        &self,
        application_id: i64,
        generated_cv_path: &str,
        format: &str,
    ) -> Result<(), rusqlite::Error> {
        self.with_conn(|conn| {
            utilities::update_application_generated_cv(conn, application_id, generated_cv_path, format)
        })
    }

    // ========================================================================
    // STATS METHODS
    // ========================================================================

    pub fn get_user_stats(&self, user_id: i64) -> Result<UserStats, rusqlite::Error> {
        self.with_conn(|conn| utilities::get_user_stats(conn, user_id))
    }

    // ========================================================================
    // ADMIN METHODS
    // ========================================================================

    pub fn list_all_cvs(&self) -> Result<Vec<(i64, String, BaseCv)>, rusqlite::Error> {
        self.with_conn(|conn| utilities::list_all_cvs(conn))
    }

    pub fn clear_all_cvs(&self) -> Result<usize, rusqlite::Error> {
        self.with_conn(|conn| utilities::clear_all_cvs(conn))
    }

    // ========================================================================
    // COVER LETTER METHODS
    // ========================================================================

    pub fn save_cover_letter(
        &self,
        application_id: i64,
        cover_letter: &str,
    ) -> Result<(), rusqlite::Error> {
        self.with_conn(|conn| utilities::save_cover_letter(conn, application_id, cover_letter))
    }

    pub fn get_cover_letter(&self, application_id: i64) -> Result<Option<String>, rusqlite::Error> {
        self.with_conn(|conn| utilities::get_cover_letter(conn, application_id))
    }

    pub fn list_applications_with_cover_letters(
        &self,
        user_id: i64,
        limit: i64,
    ) -> Result<Vec<JobApplication>, rusqlite::Error> {
        self.with_conn(|conn| utilities::list_applications_with_cover_letters(conn, user_id, limit))
    }

    // ========================================================================
    // APPLICATION REMINDER METHODS
    // ========================================================================

    pub fn set_application_reminder(
        &self,
        application_id: i64,
        reminder_date: &str,
    ) -> Result<(), rusqlite::Error> {
        self.with_conn(|conn| utilities::set_application_reminder(conn, application_id, reminder_date))
    }

    pub fn clear_application_reminder(&self, application_id: i64) -> Result<(), rusqlite::Error> {
        self.with_conn(|conn| utilities::clear_application_reminder(conn, application_id))
    }

    pub fn mark_application_reminder_sent(&self, application_id: i64) -> Result<(), rusqlite::Error> {
        self.with_conn(|conn| utilities::mark_application_reminder_sent(conn, application_id))
    }

    pub fn get_pending_application_reminders(&self) -> Result<Vec<JobApplication>, rusqlite::Error> {
        self.with_conn(|conn| utilities::get_pending_application_reminders(conn))
    }

    pub fn list_user_application_reminders(&self, user_id: i64) -> Result<Vec<JobApplication>, rusqlite::Error> {
        self.with_conn(|conn| utilities::list_user_application_reminders(conn, user_id))
    }

    // ========================================================================
    // STANDALONE REMINDER METHODS
    // ========================================================================

    pub fn create_reminder(
        &self,
        user_id: i64,
        application_id: Option<i64>,
        channel_id: i64,
        reminder_date: &str,
        message: &str,
    ) -> Result<i64, rusqlite::Error> {
        self.with_conn(|conn| {
            utilities::create_reminder(conn, user_id, application_id, channel_id, reminder_date, message)
        })
    }

    pub fn get_reminder(&self, reminder_id: i64) -> Result<Option<Reminder>, rusqlite::Error> {
        self.with_conn(|conn| utilities::get_reminder(conn, reminder_id))
    }

    pub fn list_user_reminders(&self, user_id: i64) -> Result<Vec<Reminder>, rusqlite::Error> {
        self.with_conn(|conn| utilities::list_user_reminders(conn, user_id))
    }

    pub fn delete_reminder(&self, reminder_id: i64, user_id: i64) -> Result<bool, rusqlite::Error> {
        self.with_conn(|conn| utilities::delete_reminder(conn, reminder_id, user_id))
    }

    pub fn mark_reminder_sent(&self, reminder_id: i64) -> Result<(), rusqlite::Error> {
        self.with_conn(|conn| utilities::mark_reminder_sent(conn, reminder_id))
    }

    pub fn get_pending_reminders(&self) -> Result<Vec<Reminder>, rusqlite::Error> {
        self.with_conn(|conn| utilities::get_pending_reminders(conn))
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
        }
    }
}

// Pour l'injection dans Serenity TypeMap
impl serenity::prelude::TypeMapKey for Database {
    type Value = Database;
}
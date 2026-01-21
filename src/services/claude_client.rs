use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tracing::{debug, info, error};

#[derive(Error, Debug)]
pub enum ClaudeError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("Connection error: {0}")]
    Connection(String),
}

/// HTTP Client for Claude Code server
pub struct ClaudeClient {
    base_url: String,
    client: reqwest::Client,
}

impl ClaudeClient {
    /// Create a new client
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Create client from environment variables
    pub fn from_env() -> Self {
        let base_url = std::env::var("CLAUDE_API_URL")
            .unwrap_or_else(|_| "http://claudecode:8080".to_string());
        Self::new(&base_url)
    }

    /// Check if the server is healthy
    pub async fn health_check(&self) -> Result<bool, ClaudeError> {
        let url = format!("{}/health", self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }

    /// Send a generic prompt to Claude
    pub async fn prompt(&self, prompt: &str) -> Result<String, ClaudeError> {
        let url = format!("{}/prompt", self.base_url);

        debug!("Sending prompt to {}", url);

        let response = self.client
            .post(&url)
            .json(&json!({ "prompt": prompt }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ClaudeError::Api(format!("HTTP {}: {}", status, body)));
        }

        let data: serde_json::Value = response.json().await?;

        if let Some(error) = data.get("error").and_then(|e| e.as_str()) {
            return Err(ClaudeError::Api(error.to_string()));
        }

        if let Some(response) = data.get("response").and_then(|r| r.as_str()) {
            return Ok(response.to_string());
        }

        Ok(serde_json::to_string_pretty(&data)?)
    }

    /// Synthesize a job offer
    pub async fn synthesize_job_offer(&self, job_description: &str) -> Result<JobSynthesis, ClaudeError> {
        let url = format!("{}/synthesize", self.base_url);

        info!("Synthesizing job offer");

        let response = self.client
            .post(&url)
            .json(&json!({ "job_description": job_description }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ClaudeError::Api(format!("HTTP {}: {}", status, body)));
        }

        let data: serde_json::Value = response.json().await?;

        if let Some(error) = data.get("error").and_then(|e| e.as_str()) {
            return Err(ClaudeError::Api(error.to_string()));
        }

        // Handle raw_response (fallback)
        if data.get("raw_response").is_some() {
            error!("Got raw response instead of structured data");
            return Err(ClaudeError::Api("Failed to parse job synthesis".to_string()));
        }

        let synthesis: JobSynthesis = serde_json::from_value(data)?;
        Ok(synthesis)
    }

    /// Match skills between CV and job
    pub async fn match_skills(
        &self,
        job_description: &str,
        cv_content: &str,
    ) -> Result<SkillsMatch, ClaudeError> {
        let url = format!("{}/match-skills", self.base_url);

        info!("Matching skills");

        let response = self.client
            .post(&url)
            .json(&json!({
                "job_description": job_description,
                "cv_content": cv_content
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ClaudeError::Api(format!("HTTP {}: {}", status, body)));
        }

        let data: serde_json::Value = response.json().await?;

        if let Some(error) = data.get("error").and_then(|e| e.as_str()) {
            return Err(ClaudeError::Api(error.to_string()));
        }

        if data.get("raw_response").is_some() {
            error!("Got raw response instead of structured data");
            return Err(ClaudeError::Api("Failed to parse skills match".to_string()));
        }

        let skills: SkillsMatch = serde_json::from_value(data)?;
        Ok(skills)
    }

    /// Analyze salary for a job
    pub async fn analyze_salary(
        &self,
        job_description: &str,
        location: Option<&str>,
    ) -> Result<SalaryAnalysis, ClaudeError> {
        let url = format!("{}/salary-analysis", self.base_url);

        info!("Analyzing salary");

        let response = self.client
            .post(&url)
            .json(&json!({
                "job_description": job_description,
                "location": location.unwrap_or("France")
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ClaudeError::Api(format!("HTTP {}: {}", status, body)));
        }

        let data: serde_json::Value = response.json().await?;

        if let Some(error) = data.get("error").and_then(|e| e.as_str()) {
            return Err(ClaudeError::Api(error.to_string()));
        }

        if data.get("raw_response").is_some() {
            error!("Got raw response instead of structured data");
            return Err(ClaudeError::Api("Failed to parse salary analysis".to_string()));
        }

        let salary: SalaryAnalysis = serde_json::from_value(data)?;
        Ok(salary)
    }

    /// Generate a tailored CV
    pub async fn generate_tailored_cv(
        &self,
        cv_content: &str,
        job_synthesis: &JobSynthesis,
        skills_match: &SkillsMatch,
    ) -> Result<GeneratedCv, ClaudeError> {
        let url = format!("{}/generate-cv", self.base_url);

        info!("Generating tailored CV");

        let response = self.client
            .post(&url)
            .json(&json!({
                "cv_content": cv_content,
                "job_title": job_synthesis.title,
                "company": job_synthesis.company,
                "requirements": job_synthesis.key_requirements,
                "highlights": skills_match.highlights
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ClaudeError::Api(format!("HTTP {}: {}", status, body)));
        }

        let data: serde_json::Value = response.json().await?;

        if let Some(error) = data.get("error").and_then(|e| e.as_str()) {
            return Err(ClaudeError::Api(error.to_string()));
        }

        if data.get("raw_response").is_some() {
            error!("Got raw response instead of structured data");
            return Err(ClaudeError::Api("Failed to parse generated CV".to_string()));
        }

        let cv: GeneratedCv = serde_json::from_value(data)?;
        Ok(cv)
    }
}

// ============================================================================
// Data structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSynthesis {
    pub title: String,
    pub company: String,
    pub location: String,
    pub contract_type: String,
    pub key_requirements: Vec<String>,
    #[serde(default)]
    pub responsibilities: Vec<String>,
    #[serde(default)]
    pub benefits: Vec<String>,
    #[serde(default)]
    pub salary_range: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedSkill {
    pub skill: String,
    #[serde(default)]
    pub cv_level: String,
    #[serde(default)]
    pub required: String,
    #[serde(rename = "match", default)]
    pub is_match: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingSkill {
    pub skill: String,
    #[serde(default)]
    pub importance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsMatch {
    pub match_score: u32,
    #[serde(default)]
    pub matched_skills: Vec<MatchedSkill>,
    #[serde(default)]
    pub missing_skills: Vec<MissingSkill>,
    #[serde(default)]
    pub highlights: Vec<String>,
    #[serde(default)]
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalaryAnalysis {
    pub offered_min: Option<u32>,
    pub offered_max: Option<u32>,
    #[serde(default)]
    pub market_low: u32,
    #[serde(default)]
    pub market_median: u32,
    #[serde(default)]
    pub market_high: u32,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default)]
    pub analysis: String,
    #[serde(default)]
    pub negotiation_tips: Vec<String>,
}

fn default_currency() -> String {
    "EUR".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCv {
    #[serde(default)]
    pub latex_content: String,
    #[serde(default)]
    pub adaptations: Vec<String>,
    #[serde(default)]
    pub summary: String,
}

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

#[derive(Error, Debug)]
pub enum McpError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Claude error: {0}")]
    Claude(String),
}

/// JSON-RPC 2.0 Request
#[derive(Serialize, Debug)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Deserialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<u64>,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<JsonRpcError>,
}

#[derive(Deserialize, Debug)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[serde(default)]
    data: Option<Value>,
}

/// MCP Client pour communiquer avec Claude Code
pub struct McpClient {
    host: String,
    port: u16,
    request_id: AtomicU64,
    stream: Mutex<Option<TcpStream>>,
}

impl McpClient {
    /// Crée un nouveau client MCP
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            request_id: AtomicU64::new(1),
            stream: Mutex::new(None),
        }
    }

    /// Crée un client depuis les variables d'environnement
    pub fn from_env() -> Self {
        let host = std::env::var("MCP_HOST").unwrap_or_else(|_| "claudecode".to_string());
        let port = std::env::var("MCP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);
        Self::new(&host, port)
    }

    /// Connecte au serveur MCP
    pub async fn connect(&self) -> Result<(), McpError> {
        let addr = format!("{}:{}", self.host, self.port);
        info!("Connecting to MCP server at {}", addr);

        let stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| McpError::Connection(format!("Failed to connect to {}: {}", addr, e)))?;

        *self.stream.lock().await = Some(stream);
        info!("Connected to MCP server");

        // Initialize the MCP connection
        self.initialize().await?;

        Ok(())
    }

    /// Initialize MCP session
    async fn initialize(&self) -> Result<Value, McpError> {
        let params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "discord-cv-bot",
                "version": "0.1.0"
            }
        });

        self.send_request("initialize", Some(params)).await
    }

    /// Envoie une requête JSON-RPC et attend la réponse
    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<Value, McpError> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);

        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)?;
        debug!("Sending MCP request: {}", request_json);

        let mut stream_guard = self.stream.lock().await;
        let stream = stream_guard
            .as_mut()
            .ok_or_else(|| McpError::Connection("Not connected".to_string()))?;

        // Envoyer la requête (avec newline comme délimiteur)
        stream
            .write_all(format!("{}\n", request_json).as_bytes())
            .await?;
        stream.flush().await?;

        // Lire la réponse
        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        debug!("Received MCP response: {}", response_line.trim());

        let response: JsonRpcResponse = serde_json::from_str(&response_line)?;

        if let Some(error) = response.error {
            return Err(McpError::Protocol(format!(
                "RPC error {}: {}",
                error.code, error.message
            )));
        }

        response
            .result
            .ok_or_else(|| McpError::Protocol("Empty response".to_string()))
    }

    /// Liste les outils disponibles
    pub async fn list_tools(&self) -> Result<Value, McpError> {
        self.send_request("tools/list", None).await
    }

    /// Appelle un outil MCP
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, McpError> {
        let params = json!({
            "name": name,
            "arguments": arguments
        });

        self.send_request("tools/call", Some(params)).await
    }

    /// Envoie un prompt à Claude via l'outil Bash (exécute claude -p)
    pub async fn send_prompt(&self, prompt: &str) -> Result<String, McpError> {
        // Échapper les guillemets dans le prompt
        let escaped_prompt = prompt.replace('\\', "\\\\").replace('"', "\\\"");

        let command = format!(
            "claude -p \"{}\" --output-format json",
            escaped_prompt
        );

        let result = self.call_tool("Bash", json!({ "command": command })).await?;

        // Extraire le résultat
        if let Some(content) = result.get("content") {
            if let Some(text) = content.as_str() {
                return Ok(text.to_string());
            }
            if let Some(arr) = content.as_array() {
                if let Some(first) = arr.first() {
                    if let Some(text) = first.get("text").and_then(|t| t.as_str()) {
                        return Ok(text.to_string());
                    }
                }
            }
        }

        Ok(serde_json::to_string_pretty(&result)?)
    }

    /// Synthétise une offre d'emploi
    pub async fn synthesize_job_offer(&self, job_description: &str) -> Result<JobSynthesis, McpError> {
        let prompt = format!(
            r#"Analyse cette offre d'emploi et retourne un JSON avec la structure suivante:
{{
    "title": "titre du poste",
    "company": "nom de l'entreprise",
    "location": "lieu",
    "contract_type": "type de contrat (CDI, CDD, etc.)",
    "key_requirements": ["compétence1", "compétence2"],
    "responsibilities": ["responsabilité1", "responsabilité2"],
    "benefits": ["avantage1", "avantage2"],
    "salary_range": "fourchette salariale si mentionnée",
    "summary": "résumé en 2-3 phrases"
}}

Offre d'emploi:
{}

Réponds UNIQUEMENT avec le JSON, sans autre texte."#,
            job_description
        );

        let response = self.send_prompt(&prompt).await?;

        // Parser le JSON de la réponse
        let synthesis: JobSynthesis = serde_json::from_str(&response)
            .map_err(|e| McpError::Claude(format!("Failed to parse synthesis: {} - Response: {}", e, response)))?;

        Ok(synthesis)
    }

    /// Analyse les compétences et le matching avec un CV
    pub async fn match_skills(
        &self,
        job_description: &str,
        cv_content: &str,
    ) -> Result<SkillsMatch, McpError> {
        let prompt = format!(
            r#"Compare ce CV avec cette offre d'emploi et retourne un JSON avec la structure suivante:
{{
    "match_score": 85,
    "matched_skills": [
        {{"skill": "Rust", "cv_level": "5 ans", "required": "3 ans", "match": true}}
    ],
    "missing_skills": [
        {{"skill": "Kubernetes", "importance": "nice-to-have"}}
    ],
    "highlights": ["point fort 1", "point fort 2"],
    "recommendations": ["recommandation 1", "recommandation 2"]
}}

CV:
{}

Offre d'emploi:
{}

Réponds UNIQUEMENT avec le JSON, sans autre texte."#,
            cv_content, job_description
        );

        let response = self.send_prompt(&prompt).await?;

        let skills: SkillsMatch = serde_json::from_str(&response)
            .map_err(|e| McpError::Claude(format!("Failed to parse skills match: {} - Response: {}", e, response)))?;

        Ok(skills)
    }

    /// Analyse salariale
    pub async fn analyze_salary(
        &self,
        job_description: &str,
        location: Option<&str>,
    ) -> Result<SalaryAnalysis, McpError> {
        let location_str = location.unwrap_or("France");

        let prompt = format!(
            r#"Analyse le salaire pour cette offre d'emploi et retourne un JSON avec la structure suivante:
{{
    "offered_min": 50000,
    "offered_max": 65000,
    "market_low": 48000,
    "market_median": 58000,
    "market_high": 72000,
    "currency": "EUR",
    "analysis": "Analyse détaillée du positionnement salarial",
    "negotiation_tips": ["conseil 1", "conseil 2"]
}}

Si le salaire n'est pas mentionné dans l'offre, estime-le based sur le marché.

Offre d'emploi:
{}

Localisation: {}

Réponds UNIQUEMENT avec le JSON, sans autre texte."#,
            job_description, location_str
        );

        let response = self.send_prompt(&prompt).await?;

        let salary: SalaryAnalysis = serde_json::from_str(&response)
            .map_err(|e| McpError::Claude(format!("Failed to parse salary analysis: {} - Response: {}", e, response)))?;

        Ok(salary)
    }

    /// Génère un CV adapté (retourne le contenu LaTeX)
    pub async fn generate_tailored_cv(
        &self,
        cv_content: &str,
        job_synthesis: &JobSynthesis,
        skills_match: &SkillsMatch,
    ) -> Result<GeneratedCv, McpError> {
        let prompt = format!(
            r#"Génère un CV adapté à cette offre d'emploi au format LaTeX.

CV original:
{}

Poste visé: {} chez {}
Compétences clés requises: {}
Points forts à mettre en avant: {}

Génère un CV LaTeX professionnel qui:
1. Met en avant les compétences matchées
2. Adapte le titre et le résumé au poste
3. Réorganise les expériences par pertinence
4. Utilise un template moderne

Retourne un JSON avec:
{{
    "latex_content": "\\documentclass{{article}}...",
    "adaptations": ["adaptation 1", "adaptation 2"],
    "summary": "résumé des modifications"
}}

Réponds UNIQUEMENT avec le JSON, sans autre texte."#,
            cv_content,
            job_synthesis.title,
            job_synthesis.company,
            job_synthesis.key_requirements.join(", "),
            skills_match.highlights.join(", ")
        );

        let response = self.send_prompt(&prompt).await?;

        let cv: GeneratedCv = serde_json::from_str(&response)
            .map_err(|e| McpError::Claude(format!("Failed to parse generated CV: {} - Response: {}", e, response)))?;

        Ok(cv)
    }
}

// ============================================================================
// Data structures for responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSynthesis {
    pub title: String,
    pub company: String,
    pub location: String,
    pub contract_type: String,
    pub key_requirements: Vec<String>,
    pub responsibilities: Vec<String>,
    pub benefits: Vec<String>,
    #[serde(default)]
    pub salary_range: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedSkill {
    pub skill: String,
    pub cv_level: String,
    pub required: String,
    #[serde(rename = "match")]
    pub is_match: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingSkill {
    pub skill: String,
    pub importance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsMatch {
    pub match_score: u32,
    pub matched_skills: Vec<MatchedSkill>,
    pub missing_skills: Vec<MissingSkill>,
    pub highlights: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalaryAnalysis {
    pub offered_min: Option<u32>,
    pub offered_max: Option<u32>,
    pub market_low: u32,
    pub market_median: u32,
    pub market_high: u32,
    pub currency: String,
    pub analysis: String,
    pub negotiation_tips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCv {
    pub latex_content: String,
    pub adaptations: Vec<String>,
    pub summary: String,
}

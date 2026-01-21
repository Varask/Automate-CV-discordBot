use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tracing::{debug, info};

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
    #[error("Timeout")]
    Timeout,
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
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    #[serde(default)]
    data: Option<Value>,
}

/// MCP Client pour communiquer avec Claude Code
/// Utilise le format Content-Length (comme LSP)
pub struct McpClient {
    host: String,
    port: u16,
    request_id: AtomicU64,
    stream: Mutex<Option<TcpStream>>,
    initialized: Mutex<bool>,
}

impl McpClient {
    /// Crée un nouveau client MCP
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            request_id: AtomicU64::new(1),
            stream: Mutex::new(None),
            initialized: Mutex::new(false),
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
        *self.initialized.lock().await = true;

        Ok(())
    }

    /// Ensure connection is established
    async fn ensure_connected(&self) -> Result<(), McpError> {
        let initialized = *self.initialized.lock().await;
        if !initialized {
            self.connect().await?;
        }
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

        let result = self.send_request("initialize", Some(params)).await?;

        // Send initialized notification
        self.send_notification("notifications/initialized", None).await?;

        Ok(result)
    }

    /// Encode message with Content-Length header (LSP/MCP format)
    fn encode_message(content: &str) -> Vec<u8> {
        let header = format!("Content-Length: {}\r\n\r\n", content.len());
        let mut message = header.into_bytes();
        message.extend_from_slice(content.as_bytes());
        message
    }

    /// Send a notification (no response expected)
    async fn send_notification(&self, method: &str, params: Option<Value>) -> Result<(), McpError> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let content = serde_json::to_string(&notification)?;
        let message = Self::encode_message(&content);

        let mut stream_guard = self.stream.lock().await;
        let stream = stream_guard
            .as_mut()
            .ok_or_else(|| McpError::Connection("Not connected".to_string()))?;

        stream.write_all(&message).await?;
        stream.flush().await?;

        debug!("Sent notification: {}", method);
        Ok(())
    }

    /// Envoie une requête JSON-RPC et attend la réponse (format Content-Length)
    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<Value, McpError> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);

        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        let content = serde_json::to_string(&request)?;
        let message = Self::encode_message(&content);

        debug!("Sending MCP request: {}", content);

        let mut stream_guard = self.stream.lock().await;
        let stream = stream_guard
            .as_mut()
            .ok_or_else(|| McpError::Connection("Not connected".to_string()))?;

        // Send the message
        stream.write_all(&message).await?;
        stream.flush().await?;

        // Read the response with Content-Length header
        let response = self.read_response(stream).await?;

        debug!("Received MCP response: {:?}", response);

        if let Some(error) = response.error {
            return Err(McpError::Protocol(format!(
                "RPC error {}: {}",
                error.code, error.message
            )));
        }

        response
            .result
            .ok_or_else(|| McpError::Protocol("Empty result in response".to_string()))
    }

    /// Read a response with Content-Length header
    async fn read_response(&self, stream: &mut TcpStream) -> Result<JsonRpcResponse, McpError> {
        let mut reader = BufReader::new(stream);

        // Read headers until we find Content-Length
        let mut content_length: Option<usize> = None;

        loop {
            let mut header_line = String::new();
            let bytes_read = reader.read_line(&mut header_line).await?;

            if bytes_read == 0 {
                return Err(McpError::Protocol("Connection closed".to_string()));
            }

            let trimmed = header_line.trim();

            // Empty line signals end of headers
            if trimmed.is_empty() {
                break;
            }

            // Parse Content-Length header
            if let Some(len_str) = trimmed.strip_prefix("Content-Length:") {
                content_length = Some(
                    len_str
                        .trim()
                        .parse()
                        .map_err(|_| McpError::Protocol("Invalid Content-Length".to_string()))?,
                );
            }
        }

        let content_length = content_length
            .ok_or_else(|| McpError::Protocol("Missing Content-Length header".to_string()))?;

        // Read the body
        let mut body = vec![0u8; content_length];
        reader.read_exact(&mut body).await?;

        let body_str = String::from_utf8(body)
            .map_err(|_| McpError::Protocol("Invalid UTF-8 in response".to_string()))?;

        debug!("Response body: {}", body_str);

        let response: JsonRpcResponse = serde_json::from_str(&body_str)?;
        Ok(response)
    }

    /// Liste les outils disponibles
    pub async fn list_tools(&self) -> Result<Value, McpError> {
        self.ensure_connected().await?;
        self.send_request("tools/list", None).await
    }

    /// Appelle un outil MCP
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, McpError> {
        self.ensure_connected().await?;

        let params = json!({
            "name": name,
            "arguments": arguments
        });

        self.send_request("tools/call", Some(params)).await
    }

    /// Envoie un prompt à Claude via l'outil Bash (exécute claude -p)
    pub async fn send_prompt(&self, prompt: &str) -> Result<String, McpError> {
        // Échapper les guillemets dans le prompt
        let escaped_prompt = prompt
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
            .replace('`', "\\`");

        let command = format!(
            "claude -p \"{}\" --output-format json 2>/dev/null || claude -p \"{}\"",
            escaped_prompt, escaped_prompt
        );

        let result = self.call_tool("Bash", json!({ "command": command })).await?;

        // Extraire le résultat
        self.extract_text_from_result(&result)
    }

    /// Extract text from MCP tool result
    fn extract_text_from_result(&self, result: &Value) -> Result<String, McpError> {
        // Try different response formats
        if let Some(content) = result.get("content") {
            // Array format: [{"type": "text", "text": "..."}]
            if let Some(arr) = content.as_array() {
                for item in arr {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        return Ok(text.to_string());
                    }
                }
            }
            // Direct string
            if let Some(text) = content.as_str() {
                return Ok(text.to_string());
            }
        }

        // Try direct text field
        if let Some(text) = result.get("text").and_then(|t| t.as_str()) {
            return Ok(text.to_string());
        }

        // Return the whole result as string
        Ok(serde_json::to_string_pretty(result)?)
    }

    /// Synthétise une offre d'emploi
    pub async fn synthesize_job_offer(&self, job_description: &str) -> Result<JobSynthesis, McpError> {
        let prompt = format!(
            r#"Analyse cette offre d'emploi et retourne UNIQUEMENT un JSON valide (sans texte avant ou après) avec cette structure:
{{
    "title": "titre du poste",
    "company": "nom de l'entreprise ou 'Non spécifié'",
    "location": "lieu ou 'Non spécifié'",
    "contract_type": "type de contrat (CDI, CDD, etc.) ou 'Non spécifié'",
    "key_requirements": ["compétence1", "compétence2"],
    "responsibilities": ["responsabilité1", "responsabilité2"],
    "benefits": ["avantage1", "avantage2"],
    "salary_range": "fourchette salariale si mentionnée ou null",
    "summary": "résumé en 2-3 phrases"
}}

Offre d'emploi:
{}"#,
            job_description
        );

        let response = self.send_prompt(&prompt).await?;

        // Try to extract JSON from response
        let json_str = self.extract_json_from_response(&response)?;

        let synthesis: JobSynthesis = serde_json::from_str(&json_str)
            .map_err(|e| McpError::Claude(format!("Failed to parse synthesis: {} - JSON: {}", e, json_str)))?;

        Ok(synthesis)
    }

    /// Extract JSON from a response that might contain other text
    fn extract_json_from_response(&self, response: &str) -> Result<String, McpError> {
        // Find JSON object in response
        let trimmed = response.trim();

        // If it starts with {, try to parse directly
        if trimmed.starts_with('{') {
            // Find matching closing brace
            let mut depth = 0;
            let mut end_idx = 0;
            for (i, c) in trimmed.chars().enumerate() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end_idx = i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if end_idx > 0 {
                return Ok(trimmed[..end_idx].to_string());
            }
        }

        // Try to find JSON in markdown code block
        if let Some(start) = trimmed.find("```json") {
            if let Some(end) = trimmed[start..].find("```\n").or(trimmed[start..].rfind("```")) {
                let json_start = start + 7; // "```json".len()
                let json_end = start + end;
                if json_end > json_start {
                    return Ok(trimmed[json_start..json_end].trim().to_string());
                }
            }
        }

        // Try to find any JSON object
        if let Some(start) = trimmed.find('{') {
            if let Some(end) = trimmed.rfind('}') {
                if end > start {
                    return Ok(trimmed[start..=end].to_string());
                }
            }
        }

        Err(McpError::Claude(format!("No JSON found in response: {}", response)))
    }

    /// Analyse les compétences et le matching avec un CV
    pub async fn match_skills(
        &self,
        job_description: &str,
        cv_content: &str,
    ) -> Result<SkillsMatch, McpError> {
        let prompt = format!(
            r#"Compare ce CV avec cette offre d'emploi et retourne UNIQUEMENT un JSON valide:
{{
    "match_score": 75,
    "matched_skills": [
        {{"skill": "Python", "cv_level": "3 ans", "required": "2 ans", "match": true}}
    ],
    "missing_skills": [
        {{"skill": "Kubernetes", "importance": "nice-to-have"}}
    ],
    "highlights": ["point fort 1", "point fort 2"],
    "recommendations": ["recommandation 1"]
}}

CV:
{}

Offre d'emploi:
{}"#,
            cv_content, job_description
        );

        let response = self.send_prompt(&prompt).await?;
        let json_str = self.extract_json_from_response(&response)?;

        let skills: SkillsMatch = serde_json::from_str(&json_str)
            .map_err(|e| McpError::Claude(format!("Failed to parse skills match: {} - JSON: {}", e, json_str)))?;

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
            r#"Analyse le salaire pour cette offre et retourne UNIQUEMENT un JSON valide:
{{
    "offered_min": 50000,
    "offered_max": 65000,
    "market_low": 48000,
    "market_median": 58000,
    "market_high": 72000,
    "currency": "EUR",
    "analysis": "Analyse du positionnement salarial",
    "negotiation_tips": ["conseil 1", "conseil 2"]
}}

Si le salaire n'est pas mentionné, mets null pour offered_min et offered_max.

Offre:
{}

Localisation: {}"#,
            job_description, location_str
        );

        let response = self.send_prompt(&prompt).await?;
        let json_str = self.extract_json_from_response(&response)?;

        let salary: SalaryAnalysis = serde_json::from_str(&json_str)
            .map_err(|e| McpError::Claude(format!("Failed to parse salary: {} - JSON: {}", e, json_str)))?;

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
            r#"Génère un CV adapté au format LaTeX. Retourne UNIQUEMENT un JSON valide:
{{
    "latex_content": "\\documentclass{{article}}...",
    "adaptations": ["adaptation 1", "adaptation 2"],
    "summary": "résumé des modifications"
}}

CV original:
{}

Poste: {} chez {}
Compétences requises: {}
Points forts: {}"#,
            cv_content,
            job_synthesis.title,
            job_synthesis.company,
            job_synthesis.key_requirements.join(", "),
            skills_match.highlights.join(", ")
        );

        let response = self.send_prompt(&prompt).await?;
        let json_str = self.extract_json_from_response(&response)?;

        let cv: GeneratedCv = serde_json::from_str(&json_str)
            .map_err(|e| McpError::Claude(format!("Failed to parse CV: {} - JSON: {}", e, json_str)))?;

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

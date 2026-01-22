#!/usr/bin/env python3
"""
Simple HTTP wrapper for Claude Code CLI.
Exposes claude -p as HTTP endpoints.
"""

import json
import subprocess
import os
import base64
import tempfile
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse
import traceback

# PDF extraction - try multiple libraries
PDF_EXTRACTOR = None
try:
    import pdfplumber
    PDF_EXTRACTOR = "pdfplumber"
except ImportError:
    try:
        import PyPDF2
        PDF_EXTRACTOR = "pypdf2"
    except ImportError:
        print("Warning: No PDF library available. Install pdfplumber or PyPDF2 for PDF extraction.")

PORT = int(os.environ.get("PORT", 8080))

class ClaudeHandler(BaseHTTPRequestHandler):
    def _set_headers(self, status=200, content_type="application/json"):
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()

    def _send_json(self, data, status=200):
        self._set_headers(status)
        self.wfile.write(json.dumps(data).encode())

    def _send_error(self, message, status=500):
        self._send_json({"error": message}, status)

    def do_OPTIONS(self):
        self._set_headers(200)

    def do_GET(self):
        path = urlparse(self.path).path

        if path == "/health":
            self._send_json({"status": "ok", "service": "claude-server"})
        else:
            self._send_error("Not found", 404)

    def do_POST(self):
        path = urlparse(self.path).path

        try:
            content_length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(content_length).decode("utf-8")
            data = json.loads(body) if body else {}
        except json.JSONDecodeError as e:
            self._send_error(f"Invalid JSON: {e}", 400)
            return

        try:
            if path == "/prompt":
                result = self.handle_prompt(data)
            elif path == "/synthesize":
                result = self.handle_synthesize(data)
            elif path == "/match-skills":
                result = self.handle_match_skills(data)
            elif path == "/salary-analysis":
                result = self.handle_salary_analysis(data)
            elif path == "/generate-cv":
                result = self.handle_generate_cv(data)
            elif path == "/extract-pdf":
                result = self.handle_extract_pdf(data)
            else:
                self._send_error("Not found", 404)
                return

            self._send_json(result)

        except Exception as e:
            traceback.print_exc()
            self._send_error(str(e), 500)

    def run_claude(self, prompt: str, timeout: int = 120) -> str:
        """Execute claude -p with the given prompt."""
        try:
            result = subprocess.run(
                ["claude", "-p", prompt],
                capture_output=True,
                text=True,
                timeout=timeout
            )

            if result.returncode != 0 and result.stderr:
                print(f"Claude stderr: {result.stderr}")

            return result.stdout.strip()
        except subprocess.TimeoutExpired:
            raise Exception(f"Claude timeout after {timeout}s")
        except Exception as e:
            raise Exception(f"Claude error: {e}")

    def handle_prompt(self, data: dict) -> dict:
        """Generic prompt endpoint."""
        prompt = data.get("prompt", "")
        if not prompt:
            raise ValueError("Missing 'prompt' field")

        response = self.run_claude(prompt)
        return {"response": response}

    def handle_synthesize(self, data: dict) -> dict:
        """Synthesize a job offer."""
        job_description = data.get("job_description", "")
        if not job_description:
            raise ValueError("Missing 'job_description' field")

        prompt = f'''Analyse cette offre d'emploi et retourne UNIQUEMENT un JSON valide:
{{
    "title": "titre du poste",
    "company": "nom de l'entreprise ou Non spécifié",
    "location": "lieu ou Non spécifié",
    "contract_type": "CDI/CDD/etc ou Non spécifié",
    "key_requirements": ["compétence1", "compétence2"],
    "responsibilities": ["responsabilité1"],
    "benefits": ["avantage1"],
    "salary_range": null,
    "summary": "résumé en 2-3 phrases"
}}

Offre:
{job_description}'''

        response = self.run_claude(prompt)
        return self.extract_json(response)

    def handle_match_skills(self, data: dict) -> dict:
        """Match CV skills with job requirements."""
        job_description = data.get("job_description", "")
        cv_content = data.get("cv_content", "CV non fourni")

        prompt = f'''Compare ce CV avec cette offre et retourne UNIQUEMENT un JSON valide:
{{
    "match_score": 75,
    "matched_skills": [{{"skill": "Python", "cv_level": "3 ans", "required": "2 ans", "match": true}}],
    "missing_skills": [{{"skill": "Kubernetes", "importance": "nice-to-have"}}],
    "highlights": ["point fort 1"],
    "recommendations": ["recommandation 1"]
}}

CV:
{cv_content}

Offre:
{job_description}'''

        response = self.run_claude(prompt)
        return self.extract_json(response)

    def handle_salary_analysis(self, data: dict) -> dict:
        """Analyze salary for a job."""
        job_description = data.get("job_description", "")
        location = data.get("location", "France")

        prompt = f'''Analyse le salaire pour cette offre et retourne UNIQUEMENT un JSON valide:
{{
    "offered_min": null,
    "offered_max": null,
    "market_low": 45000,
    "market_median": 55000,
    "market_high": 70000,
    "currency": "EUR",
    "analysis": "Analyse du positionnement salarial",
    "negotiation_tips": ["conseil 1"]
}}

Offre:
{job_description}

Localisation: {location}'''

        response = self.run_claude(prompt)
        return self.extract_json(response)

    def handle_generate_cv(self, data: dict) -> dict:
        """Generate a tailored CV."""
        cv_content = data.get("cv_content", "")
        job_title = data.get("job_title", "")
        company = data.get("company", "")
        requirements = data.get("requirements", [])
        highlights = data.get("highlights", [])

        prompt = f'''Génère un CV adapté pour le poste. Retourne UNIQUEMENT un JSON valide avec ces champs:
- "adaptations": liste des modifications apportées
- "summary": résumé des adaptations (2-3 phrases)
- "cv_text": le CV adapté en format texte structuré (PAS de LaTeX)

Format attendu:
{{
    "adaptations": ["Mise en avant de X", "Reformulation de Y"],
    "summary": "CV adapté pour mettre en valeur...",
    "cv_text": "NOM PRENOM\\n\\nPROFIL\\n..."
}}

CV original:
{cv_content}

Poste: {job_title} chez {company}
Compétences requises: {", ".join(requirements)}
Points forts à valoriser: {", ".join(highlights)}

IMPORTANT: Retourne UNIQUEMENT le JSON, sans markdown, sans commentaires.'''

        response = self.run_claude(prompt, timeout=180)
        result = self.extract_json(response)

        # Fallback: si on a raw_response, essayer de construire une réponse valide
        if "raw_response" in result:
            return {
                "latex_content": "",
                "cv_text": result.get("raw_response", ""),
                "adaptations": ["CV généré (format brut)"],
                "summary": "Le CV a été généré mais le parsing JSON a échoué. Contenu disponible en texte brut."
            }

        # Compatibilité: renommer cv_text en latex_content si absent
        if "cv_text" in result and "latex_content" not in result:
            result["latex_content"] = result["cv_text"]

        return result

    def handle_extract_pdf(self, data: dict) -> dict:
        """Extract text from a PDF file."""
        pdf_base64 = data.get("pdf_base64", "")
        if not pdf_base64:
            raise ValueError("Missing 'pdf_base64' field")

        if not PDF_EXTRACTOR:
            return {
                "success": False,
                "error": "No PDF library available on server",
                "text": ""
            }

        try:
            # Decode base64 to bytes
            pdf_bytes = base64.b64decode(pdf_base64)

            # Write to temp file
            with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as tmp:
                tmp.write(pdf_bytes)
                tmp_path = tmp.name

            text = ""
            try:
                if PDF_EXTRACTOR == "pdfplumber":
                    import pdfplumber
                    with pdfplumber.open(tmp_path) as pdf:
                        for page in pdf.pages:
                            page_text = page.extract_text()
                            if page_text:
                                text += page_text + "\n\n"
                elif PDF_EXTRACTOR == "pypdf2":
                    import PyPDF2
                    with open(tmp_path, "rb") as f:
                        reader = PyPDF2.PdfReader(f)
                        for page in reader.pages:
                            page_text = page.extract_text()
                            if page_text:
                                text += page_text + "\n\n"
            finally:
                # Clean up temp file
                os.unlink(tmp_path)

            return {
                "success": True,
                "text": text.strip(),
                "extractor": PDF_EXTRACTOR
            }

        except Exception as e:
            return {
                "success": False,
                "error": str(e),
                "text": ""
            }

    def extract_json(self, response: str) -> dict:
        """Extract JSON from Claude's response."""
        response = response.strip()

        # Try direct parse
        try:
            return json.loads(response)
        except json.JSONDecodeError:
            pass

        # Find JSON in markdown code block
        if "```json" in response:
            start = response.find("```json") + 7
            end = response.find("```", start)
            if end > start:
                try:
                    return json.loads(response[start:end].strip())
                except json.JSONDecodeError:
                    pass

        # Find any JSON object
        start = response.find("{")
        end = response.rfind("}") + 1
        if start >= 0 and end > start:
            try:
                return json.loads(response[start:end])
            except json.JSONDecodeError:
                pass

        # Return raw response wrapped
        return {"raw_response": response}

    def log_message(self, format, *args):
        print(f"[{self.log_date_time_string()}] {format % args}")


def main():
    server = HTTPServer(("0.0.0.0", PORT), ClaudeHandler)
    print(f"🚀 Claude HTTP Server running on port {PORT}")
    print(f"📄 PDF Extractor: {PDF_EXTRACTOR or 'None (install pdfplumber)'}")
    print(f"Endpoints:")
    print(f"  GET  /health           - Health check")
    print(f"  POST /prompt           - Generic prompt")
    print(f"  POST /synthesize       - Job synthesis")
    print(f"  POST /match-skills     - Skills matching")
    print(f"  POST /salary-analysis  - Salary analysis")
    print(f"  POST /generate-cv      - CV generation")
    print(f"  POST /extract-pdf      - PDF text extraction")

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down...")
        server.shutdown()


if __name__ == "__main__":
    main()

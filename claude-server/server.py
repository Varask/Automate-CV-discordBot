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

# PDF generation
PDF_GENERATOR = None
try:
    from reportlab.lib.pagesizes import A4
    from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
    from reportlab.lib.units import cm
    from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, HRFlowable
    from reportlab.lib.enums import TA_LEFT, TA_CENTER
    from reportlab.pdfbase import pdfmetrics
    from reportlab.pdfbase.ttfonts import TTFont
    from reportlab.lib.colors import HexColor
    PDF_GENERATOR = "reportlab"
except ImportError:
    print("Warning: reportlab not available. Install reportlab for PDF generation.")

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
            elif path == "/generate-pdf":
                result = self.handle_generate_pdf(data)
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

        prompt = f'''Génère un CV professionnel adapté pour le poste. Retourne UNIQUEMENT un JSON valide.

STRUCTURE OBLIGATOIRE du cv_text (utilise ces sections exactes):
1. PROFIL - 2-3 phrases percutantes adaptées au poste
2. COMPÉTENCES CLÉS - Format "Catégorie: compétence1, compétence2" (une ligne par catégorie)
3. EXPÉRIENCE PROFESSIONNELLE - Chaque entrée: "Dates | Poste | Entreprise | Lieu" puis bullets "• accomplissement"
4. FORMATION - Chaque entrée: "Dates | Diplôme | École"
5. CENTRES D'INTÉRÊT - Une ligne avec les intérêts pertinents

FORMAT JSON ATTENDU:
{{
    "adaptations": ["Modification 1", "Modification 2"],
    "summary": "Résumé des adaptations en 2 phrases",
    "cv_text": "PROFIL\\nTexte du profil adapté...\\n\\nCOMPÉTENCES CLÉS\\nLangages: C, C++, Python\\nSystèmes: Linux, RTOS\\n\\nEXPÉRIENCE PROFESSIONNELLE\\n2023-2025 | Ingénieur Dev | Entreprise | Lieu\\n• Accomplissement 1\\n• Accomplissement 2\\n\\nFORMATION\\n2020-2025 | Diplôme | École\\n\\nCENTRES D'INTÉRÊT\\nIntérêt1, Intérêt2"
}}

CV ORIGINAL:
{cv_content}

POSTE VISÉ: {job_title} chez {company}
COMPÉTENCES REQUISES: {", ".join(requirements)}
POINTS FORTS À VALORISER: {", ".join(highlights)}

RÈGLES:
- Adapte le profil pour cibler spécifiquement ce poste
- Réorganise les compétences pour mettre en avant celles demandées
- Reformule les expériences pour matcher les requirements
- Reste factuel et concis (max 1 page)
- Retourne UNIQUEMENT le JSON, sans markdown'''

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
            return {
                "success": False,
                "error": "Missing 'pdf_base64' field",
                "text": ""
            }

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

    def handle_generate_pdf(self, data: dict) -> dict:
        """Generate a professional PDF CV inspired by ModernCV template."""
        cv_content = data.get("cv_content", "")
        name = data.get("name", "Candidat")
        job_title = data.get("job_title", "")
        company = data.get("company", "")

        if not cv_content:
            return {
                "success": False,
                "error": "Missing 'cv_content' field",
                "pdf_base64": ""
            }

        if not PDF_GENERATOR:
            return {
                "success": False,
                "error": "PDF generator not available. Install reportlab.",
                "pdf_base64": ""
            }

        try:
            from reportlab.lib.pagesizes import A4
            from reportlab.lib.styles import ParagraphStyle
            from reportlab.lib.units import cm, mm
            from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle
            from reportlab.lib.enums import TA_LEFT, TA_CENTER, TA_RIGHT, TA_JUSTIFY
            from reportlab.lib.colors import HexColor, black, white
            from reportlab.platypus import HRFlowable
            from reportlab.lib import colors

            # Colors inspired by ModernCV blue theme
            PRIMARY_COLOR = HexColor('#2E5090')  # Blue
            SECONDARY_COLOR = HexColor('#404040')  # Dark gray
            LIGHT_GRAY = HexColor('#808080')
            VERY_LIGHT = HexColor('#F5F5F5')

            # Create temp file for PDF
            with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as tmp:
                tmp_path = tmp.name

            # Create PDF document with tighter margins
            doc = SimpleDocTemplate(
                tmp_path,
                pagesize=A4,
                rightMargin=1.2*cm,
                leftMargin=1.2*cm,
                topMargin=1*cm,
                bottomMargin=1*cm
            )

            # Define professional styles
            styles = {}

            # Name style - large and prominent
            styles['name'] = ParagraphStyle(
                'Name',
                fontSize=22,
                textColor=PRIMARY_COLOR,
                spaceAfter=2,
                fontName='Helvetica-Bold',
                alignment=TA_LEFT
            )

            # Title/Role style
            styles['title'] = ParagraphStyle(
                'Title',
                fontSize=11,
                textColor=LIGHT_GRAY,
                spaceAfter=8,
                fontName='Helvetica-Oblique',
                alignment=TA_LEFT
            )

            # Section header style
            styles['section'] = ParagraphStyle(
                'Section',
                fontSize=11,
                textColor=PRIMARY_COLOR,
                spaceBefore=10,
                spaceAfter=4,
                fontName='Helvetica-Bold',
                alignment=TA_LEFT
            )

            # Subsection/Job title style
            styles['subsection'] = ParagraphStyle(
                'Subsection',
                fontSize=10,
                textColor=SECONDARY_COLOR,
                spaceBefore=6,
                spaceAfter=2,
                fontName='Helvetica-Bold',
                alignment=TA_LEFT
            )

            # Company/Date info
            styles['info'] = ParagraphStyle(
                'Info',
                fontSize=9,
                textColor=LIGHT_GRAY,
                spaceAfter=3,
                fontName='Helvetica-Oblique',
                alignment=TA_LEFT
            )

            # Body text style - compact
            styles['body'] = ParagraphStyle(
                'Body',
                fontSize=9,
                textColor=SECONDARY_COLOR,
                spaceAfter=2,
                leading=11,
                fontName='Helvetica',
                alignment=TA_JUSTIFY
            )

            # Bullet style - compact
            styles['bullet'] = ParagraphStyle(
                'Bullet',
                fontSize=9,
                textColor=SECONDARY_COLOR,
                leftIndent=12,
                spaceAfter=1,
                leading=11,
                fontName='Helvetica',
                bulletIndent=0,
                alignment=TA_LEFT
            )

            # Skill item style
            styles['skill'] = ParagraphStyle(
                'Skill',
                fontSize=9,
                textColor=SECONDARY_COLOR,
                spaceAfter=1,
                leading=11,
                fontName='Helvetica',
                alignment=TA_LEFT
            )

            # Skill label style
            styles['skill_label'] = ParagraphStyle(
                'SkillLabel',
                fontSize=9,
                textColor=PRIMARY_COLOR,
                fontName='Helvetica-Bold',
                alignment=TA_LEFT
            )

            # Build content
            story = []

            # === HEADER ===
            story.append(Paragraph(name, styles['name']))

            # Adapted title
            if job_title:
                adapted_title = f"CV adapté pour : {job_title}"
                if company:
                    adapted_title += f" — {company}"
                story.append(Paragraph(adapted_title, styles['title']))

            # Horizontal line under header
            story.append(HRFlowable(width="100%", thickness=2, color=PRIMARY_COLOR, spaceAfter=8))

            # === PARSE CV CONTENT ===
            lines = cv_content.split('\n')
            current_section = None
            section_content = []

            def is_section_header(line):
                """Detect section headers"""
                line_clean = line.strip().rstrip(':')
                section_keywords = [
                    'PROFIL', 'PROFILE', 'RÉSUMÉ', 'SUMMARY', 'OBJECTIF',
                    'COMPÉTENCES', 'SKILLS', 'COMPETENCES', 'COMPÉTENCES CLÉS',
                    'EXPÉRIENCE', 'EXPERIENCE', 'EXPÉRIENCES', 'PARCOURS',
                    'FORMATION', 'EDUCATION', 'ÉTUDES', 'DIPLÔMES',
                    'CENTRES', 'INTÉRÊTS', 'INTERESTS', 'HOBBIES', 'LOISIRS',
                    'LANGUES', 'LANGUAGES', 'CERTIFICATIONS', 'PROJETS', 'PROJECTS',
                    'COORDONNÉES', 'CONTACT', 'INFORMATIONS'
                ]
                return (
                    line_clean.isupper() or
                    any(kw in line_clean.upper() for kw in section_keywords) or
                    (len(line_clean) < 40 and line.strip().endswith(':'))
                )

            def is_job_entry(line):
                """Detect job/education entry headers"""
                # Contains year pattern like 2020-2025 or 2024
                import re
                return bool(re.search(r'\b(19|20)\d{2}\b', line)) and len(line) < 100

            for line in lines:
                line = line.strip()
                if not line:
                    continue

                # Escape HTML
                line_safe = line.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;')

                if is_section_header(line):
                    # New section
                    story.append(Spacer(1, 6))
                    section_title = line.rstrip(':').upper()
                    story.append(Paragraph(section_title, styles['section']))
                    story.append(HRFlowable(width="25%", thickness=1, color=PRIMARY_COLOR, spaceAfter=4))
                    current_section = section_title

                elif is_job_entry(line) and current_section and ('EXPÉRIENCE' in current_section.upper() or 'EXPERIENCE' in current_section.upper() or 'FORMATION' in current_section.upper() or 'EDUCATION' in current_section.upper()):
                    # Job or education entry
                    story.append(Spacer(1, 4))
                    story.append(Paragraph(line_safe, styles['subsection']))

                elif line.startswith('•') or line.startswith('-') or line.startswith('*') or line.startswith('–'):
                    # Bullet point
                    text = line.lstrip('•-*– ').strip()
                    story.append(Paragraph(f"• {text}", styles['bullet']))

                elif ':' in line and len(line.split(':')[0]) < 25 and current_section and 'COMPÉTENCE' in current_section.upper():
                    # Skill entry like "Langages: C, C++, Python"
                    parts = line.split(':', 1)
                    if len(parts) == 2:
                        label = parts[0].strip()
                        value = parts[1].strip()
                        # Create a mini table for alignment
                        skill_data = [[
                            Paragraph(f"<b>{label}</b>", styles['skill_label']),
                            Paragraph(value, styles['skill'])
                        ]]
                        skill_table = Table(skill_data, colWidths=[3*cm, 14*cm])
                        skill_table.setStyle(TableStyle([
                            ('VALIGN', (0, 0), (-1, -1), 'TOP'),
                            ('LEFTPADDING', (0, 0), (-1, -1), 0),
                            ('RIGHTPADDING', (0, 0), (-1, -1), 4),
                            ('TOPPADDING', (0, 0), (-1, -1), 1),
                            ('BOTTOMPADDING', (0, 0), (-1, -1), 1),
                        ]))
                        story.append(skill_table)
                    else:
                        story.append(Paragraph(line_safe, styles['body']))
                else:
                    # Regular text
                    story.append(Paragraph(line_safe, styles['body']))

            # Build PDF
            doc.build(story)

            # Read and encode PDF
            with open(tmp_path, "rb") as f:
                pdf_bytes = f.read()

            # Clean up
            os.unlink(tmp_path)

            # Encode to base64
            pdf_base64 = base64.b64encode(pdf_bytes).decode('utf-8')

            return {
                "success": True,
                "pdf_base64": pdf_base64,
                "size": len(pdf_bytes)
            }

        except Exception as e:
            traceback.print_exc()
            return {
                "success": False,
                "error": str(e),
                "pdf_base64": ""
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
    print(f"📄 PDF Generator: {PDF_GENERATOR or 'None (install reportlab)'}")
    print(f"Endpoints:")
    print(f"  GET  /health           - Health check")
    print(f"  POST /prompt           - Generic prompt")
    print(f"  POST /synthesize       - Job synthesis")
    print(f"  POST /match-skills     - Skills matching")
    print(f"  POST /salary-analysis  - Salary analysis")
    print(f"  POST /generate-cv      - CV generation")
    print(f"  POST /extract-pdf      - PDF text extraction")
    print(f"  POST /generate-pdf     - PDF generation from CV content")

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down...")
        server.shutdown()


if __name__ == "__main__":
    main()

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
        """Generate a tailored CV in LaTeX ModernCV format."""
        cv_content = data.get("cv_content", "")
        job_title = data.get("job_title", "")
        company = data.get("company", "")
        requirements = data.get("requirements", [])
        highlights = data.get("highlights", [])
        fit_level = data.get("fit_level", 1)  # 1=standard, 2=modéré, 3=laxiste
        language = data.get("language", "fr")  # fr, en, es, de, etc.

        # Mapping des langues
        language_config = {
            "fr": {
                "name": "français",
                "sections": {
                    "profile": "Profil",
                    "skills": "Compétences clés",
                    "experience": "Expérience professionnelle",
                    "education": "Formation",
                    "interests": "Centres d'intérêt"
                }
            },
            "en": {
                "name": "English",
                "sections": {
                    "profile": "Profile",
                    "skills": "Key Skills",
                    "experience": "Professional Experience",
                    "education": "Education",
                    "interests": "Interests"
                }
            },
            "es": {
                "name": "español",
                "sections": {
                    "profile": "Perfil",
                    "skills": "Competencias clave",
                    "experience": "Experiencia profesional",
                    "education": "Formación",
                    "interests": "Intereses"
                }
            },
            "de": {
                "name": "Deutsch",
                "sections": {
                    "profile": "Profil",
                    "skills": "Kernkompetenzen",
                    "experience": "Berufserfahrung",
                    "education": "Ausbildung",
                    "interests": "Interessen"
                }
            }
        }

        lang_cfg = language_config.get(language, language_config["fr"])

        # Configuration du niveau d'adaptation (fit)
        fit_config = {
            1: {
                "name": "Standard",
                "description": "Adaptation légère - garde le CV proche de l'original",
                "instructions": """NIVEAU STANDARD (conservateur):
- Reformule légèrement les tâches pour utiliser le vocabulaire de l'offre
- Garde la structure et le contenu très proches de l'original
- Ne modifie que les formulations mineures
- Priorise les expériences pertinentes mais garde toutes les tâches principales"""
            },
            2: {
                "name": "Modéré",
                "description": "Adaptation modérée - reformule activement pour matcher",
                "instructions": """NIVEAU MODÉRÉ (équilibré):
- Reformule activement les tâches pour matcher les compétences demandées
- Transpose les compétences transférables vers le domaine cible
- Réorganise les bullets pour mettre en avant les plus pertinentes
- Peut omettre les tâches peu pertinentes pour gagner de la place
- Ajoute des métriques et contexte quand c'est honnêtement applicable"""
            },
            3: {
                "name": "Laxiste",
                "description": "Adaptation maximale - reformulation créative tout en restant honnête",
                "instructions": """NIVEAU LAXISTE (créatif):
- Reformulation créative et poussée des tâches
- Transpose agressivement les compétences vers le nouveau domaine
- Reformule complètement les bullets pour coller au maximum à l'offre
- Mets en avant les aspects les plus pertinents même si mineurs dans l'original
- Utilise le vocabulaire exact de l'offre autant que possible
- Peut fusionner ou réinterpréter des expériences pour plus de pertinence
- Reste HONNÊTE: ne jamais inventer d'expériences ou compétences inexistantes"""
            }
        }

        fit_cfg = fit_config.get(fit_level, fit_config[1])

        prompt = f'''Analyse ce CV et génère une version adaptée au poste visé.

CV SOURCE:
{cv_content}

POSTE VISÉ: {job_title}
ENTREPRISE: {company}
COMPÉTENCES DEMANDÉES: {", ".join(requirements[:8]) if requirements else "Non spécifiées"}
POINTS FORTS IDENTIFIÉS: {", ".join(highlights[:5]) if highlights else "À identifier"}

LANGUE DE SORTIE: {lang_cfg["name"]} (tout le contenu du CV doit être dans cette langue)
NIVEAU D'ADAPTATION: {fit_cfg["name"]} - {fit_cfg["description"]}

{fit_cfg["instructions"]}

Retourne UNIQUEMENT un JSON valide avec cette structure exacte:
{{
    "personal": {{
        "firstname": "Prénom",
        "lastname": "NOM",
        "title": "Titre professionnel adapté au poste -- Domaine",
        "address": "Ville, Pays",
        "mobility": "Mobilité nationale/internationale",
        "phone": "+33 X XX XX XX XX",
        "email": "email@example.com",
        "github": "username ou null",
        "linkedin": "username ou null"
    }},
    "profile": "Paragraphe de 3-4 phrases décrivant le profil adapté au poste. Utiliser **gras** pour les mots-clés importants.",
    "skills": [
        {{"category": "Langages", "items": "C, C++, Python, Rust"}},
        {{"category": "Systèmes", "items": "Linux embarqué, RTOS, temps réel"}},
        {{"category": "Outils", "items": "Git, CMake, Docker, CI/CD"}},
        {{"category": "Langues", "items": "Français (natif), Anglais (C1)"}}
    ],
    "experience": [
        {{
            "dates": "2023--2025",
            "title": "Titre du poste",
            "company": "Entreprise",
            "location": "Ville",
            "description": "Description courte optionnelle",
            "bullets": [
                "Accomplissement 1 aligné avec le poste visé",
                "Accomplissement 2 avec résultats mesurables",
                "Accomplissement 3 technique pertinent"
            ]
        }}
    ],
    "education": [
        {{
            "dates": "2020--2025",
            "degree": "Diplôme",
            "school": "École/Université",
            "details": "Spécialisation ou détails"
        }}
    ],
    "interests": "Liste des centres d'intérêt pertinents séparés par des virgules",
    "adaptations": ["Modification 1", "Modification 2"],
    "summary": "Résumé des adaptations en 1-2 phrases"
}}

RÈGLES IMPORTANTES:
1. Adapte le TITRE au poste visé (ex: "Ingénieur Développement C/C++ -- Aéronautique")
2. PROFILE: 3-4 phrases percutantes, utilise **gras** pour les mots-clés de l'offre
3. SKILLS: 6-10 catégories maximum, priorise celles demandées dans l'offre
4. EXPERIENCE: MAX 3 postes les plus pertinents, 3-5 bullets par poste
5. EDUCATION: MAX 3 entrées
6. Garde les dates au format "AAAA--AAAA" (avec double tiret)

ADAPTATION DES TÂCHES - RÈGLES CRITIQUES:
Tu dois REFORMULER LATÉRALEMENT les bullets/tâches pour les aligner avec le poste visé:

a) GARDE l'essence de l'expérience mais REFORMULE avec le vocabulaire de l'offre
   - Si l'offre demande "CI/CD" et le CV dit "automatisation des déploiements" → reformule en "mise en place de pipelines CI/CD"
   - Si l'offre demande "travail en équipe Agile" et le CV mentionne des sprints → mets en avant "collaboration en méthodologie Agile/Scrum"

b) TRANSPOSE les compétences transférables vers le domaine cible
   - Développement web → poste embarqué: "optimisation des performances" reste pertinent
   - Backend → DevOps: "gestion de bases de données" → "administration et monitoring d'infrastructures de données"

c) METS EN AVANT les aspects pertinents, MINIMISE ou OMETS les autres
   - Si le poste est orienté leadership: développe les aspects coordination, mentorat, revue de code
   - Si le poste est technique: développe les détails techniques, technologies, résultats mesurables

d) UTILISE LES MOTS-CLÉS EXACTS de l'offre quand c'est honnêtement applicable
   - Intègre naturellement les termes: {", ".join(requirements[:5]) if requirements else "de l'offre"}

e) QUANTIFIE et CONTEXTUALISE pour le secteur visé
   - Ajoute des métriques quand possible (%, temps, volume)
   - Adapte le contexte: "application mobile" → "application critique temps réel" si pertinent

EXEMPLES DE REFORMULATION LATÉRALE:
- "Développement de scripts Python" → "Automatisation et scripting Python pour l'industrialisation des processus"
- "Maintenance d'applications" → "Évolution et optimisation d'applications en production"
- "Travail avec l'équipe QA" → "Collaboration étroite avec les équipes qualité pour assurer la robustesse du code"
- "Création de documentation" → "Rédaction de spécifications techniques et documentation d'architecture"

NE PAS INVENTER d'expériences, mais reformuler honnêtement celles existantes pour maximiser leur pertinence.'''

        response = self.run_claude(prompt, timeout=180)
        result = self.extract_json(response)

        # Si parsing échoué, retourner le contenu brut
        if "raw_response" in result:
            return {
                "latex_content": "",
                "cv_text": result.get("raw_response", ""),
                "adaptations": ["CV généré (format brut)"],
                "summary": "Le CV a été généré mais le parsing JSON a échoué."
            }

        # Générer le LaTeX ModernCV
        try:
            latex_content = self._build_moderncv_latex(result)
            cv_text = self._build_cv_text(result)

            return {
                "latex_content": latex_content,
                "cv_text": cv_text,
                "adaptations": result.get("adaptations", []),
                "summary": result.get("summary", "CV adapté généré avec succès.")
            }
        except Exception as e:
            print(f"Error building LaTeX: {e}")
            traceback.print_exc()
            return {
                "latex_content": "",
                "cv_text": str(result),
                "adaptations": ["Erreur lors de la génération LaTeX"],
                "summary": f"Erreur: {str(e)}"
            }

    def _build_moderncv_latex(self, data: dict) -> str:
        """Build a complete ModernCV LaTeX document from structured data."""
        personal = data.get("personal", {})
        profile = data.get("profile", "")
        skills = data.get("skills", [])
        experience = data.get("experience", [])
        education = data.get("education", [])
        interests = data.get("interests", "")

        # Escape LaTeX special characters
        def esc(text):
            if not text:
                return ""
            text = str(text)
            replacements = [
                ('\\', '\\textbackslash{}'),
                ('&', '\\&'),
                ('%', '\\%'),
                ('$', '\\$'),
                ('#', '\\#'),
                ('_', '\\_'),
                ('{', '\\{'),
                ('}', '\\}'),
                ('~', '\\textasciitilde{}'),
                ('^', '\\textasciicircum{}'),
            ]
            for old, new in replacements:
                text = text.replace(old, new)
            # Convert **bold** to \textbf{bold}
            import re
            text = re.sub(r'\*\*([^*]+)\*\*', r'\\textbf{\1}', text)
            return text

        # Build header
        firstname = esc(personal.get("firstname", "Prénom"))
        lastname = esc(personal.get("lastname", "NOM"))
        title = esc(personal.get("title", ""))
        address = esc(personal.get("address", "France"))
        mobility = esc(personal.get("mobility", ""))
        phone = esc(personal.get("phone", ""))
        email = personal.get("email", "")  # Don't escape email
        github = personal.get("github", "")
        linkedin = personal.get("linkedin", "")

        # Build skills section
        skills_latex = ""
        for skill in skills:
            cat = esc(skill.get("category", ""))
            items = esc(skill.get("items", ""))
            if cat and items:
                skills_latex += f"\\cvitem{{{cat}}}{{{items}}}\n"

        # Build experience section
        experience_latex = ""
        for exp in experience:
            dates = esc(exp.get("dates", ""))
            exp_title = esc(exp.get("title", ""))
            company = esc(exp.get("company", ""))
            location = esc(exp.get("location", ""))
            description = esc(exp.get("description", ""))
            bullets = exp.get("bullets", [])

            bullets_latex = ""
            if bullets:
                bullets_latex = "\\begin{itemize}\n"
                for b in bullets:
                    bullets_latex += f"\\item {esc(b)}\n"
                bullets_latex += "\\end{itemize}"

            experience_latex += f"""\\cventry{{{dates}}}{{{exp_title}}}{{{company}}}{{{location}}}{{}}{{
{description}
{bullets_latex}}}

"""

        # Build education section
        education_latex = ""
        for edu in education:
            dates = esc(edu.get("dates", ""))
            degree = esc(edu.get("degree", ""))
            school = esc(edu.get("school", ""))
            details = esc(edu.get("details", ""))
            education_latex += f"\\cventry{{{dates}}}{{{degree}}}{{{school}}}{{}}{{}}{{{details}}}\n\n"

        # Build social links
        social_latex = ""
        if github:
            social_latex += f"\\social[github]{{{github}}}\n"
        if linkedin:
            social_latex += f"\\social[linkedin]{{{linkedin}}}\n"

        # Build complete document
        latex = f"""\\documentclass[a4paper,11pt]{{moderncv}}
\\moderncvstyle{{classic}}
\\moderncvcolor{{blue}}

\\usepackage[scale=0.90]{{geometry}}
\\usepackage[utf8]{{inputenc}}
\\usepackage[T1]{{fontenc}}
\\usepackage{{hyperref}}

\\name{{{firstname}}}{{{lastname}}}
\\title{{{title}}}
\\address{{{address}}}{{{mobility}}}{{}}
\\phone[mobile]{{{phone}}}
\\email{{{email}}}
{social_latex}

\\begin{{document}}
\\makecvtitle

\\section{{Profil}}
{esc(profile)}

\\section{{Compétences clés}}
{skills_latex}

\\section{{Expérience professionnelle}}
{experience_latex}

\\section{{Formation}}
{education_latex}

\\section{{Centres d'intérêt}}
{esc(interests)}

\\end{{document}}
"""
        return latex

    def _build_cv_text(self, data: dict) -> str:
        """Build a plain text version of the CV for display."""
        personal = data.get("personal", {})
        profile = data.get("profile", "")
        skills = data.get("skills", [])
        experience = data.get("experience", [])
        education = data.get("education", [])
        interests = data.get("interests", "")

        lines = []

        # Header
        lines.append(f"[NOM]")
        lines.append(f"{personal.get('firstname', '')} {personal.get('lastname', '')}")
        lines.append("")
        lines.append(f"[TITRE]")
        lines.append(personal.get("title", ""))
        lines.append("")

        # Profile
        lines.append("[PROFIL]")
        lines.append(profile.replace("**", ""))
        lines.append("")

        # Skills
        lines.append("[COMPETENCES]")
        for skill in skills:
            lines.append(f"{skill.get('category', '')}|{skill.get('items', '')}")
        lines.append("")

        # Experience
        lines.append("[EXPERIENCE]")
        for exp in experience:
            lines.append(f"{exp.get('dates', '')}|{exp.get('title', '')}|{exp.get('company', '')}|{exp.get('location', '')}")
            for b in exp.get("bullets", []):
                lines.append(f"- {b}")
            lines.append("")

        # Education
        lines.append("[FORMATION]")
        for edu in education:
            lines.append(f"{edu.get('dates', '')}|{edu.get('degree', '')}|{edu.get('school', '')}|{edu.get('details', '')}")
        lines.append("")

        # Interests
        lines.append("[INTERETS]")
        lines.append(interests)

        return "\n".join(lines)

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
        """Generate PDF from CV content. Tries reportlab first (more reliable), LaTeX as option."""
        cv_content = data.get("cv_content", "")
        name = data.get("name", "Candidat")
        job_title = data.get("job_title", "")
        company = data.get("company", "")
        prefer_latex = data.get("prefer_latex", False)

        if not cv_content:
            return {"success": False, "error": "Missing 'cv_content' field", "pdf_base64": ""}

        try:
            # Parse CV content into sections
            sections = self._parse_cv_sections(cv_content, name, job_title)
            print(f"Parsed sections: name={sections['name']}, title={sections['title']}")
            print(f"  - profil: {len(sections['profil'])} lines")
            print(f"  - competences: {len(sections['competences'])} lines")
            print(f"  - experience: {len(sections['experience'])} lines")
            print(f"  - formation: {len(sections['formation'])} lines")

            pdf_bytes = None
            method_used = None
            latex_error = None
            reportlab_error = None

            if prefer_latex:
                # Try LaTeX first if explicitly requested
                try:
                    latex_code = self._generate_latex(sections, job_title, company)
                    pdf_bytes = self._compile_latex(latex_code)
                    if pdf_bytes:
                        method_used = "latex"
                except Exception as e:
                    latex_error = str(e)
                    print(f"LaTeX failed: {e}")

            # Try reportlab (default, more reliable)
            if not pdf_bytes:
                try:
                    pdf_bytes = self._generate_pdf_reportlab(sections, job_title, company)
                    if pdf_bytes:
                        method_used = "reportlab"
                except Exception as e:
                    reportlab_error = str(e)
                    print(f"reportlab failed: {e}")
                    traceback.print_exc()

            # If reportlab failed and we haven't tried LaTeX yet, try it
            if not pdf_bytes and not prefer_latex:
                try:
                    print("reportlab failed, trying LaTeX fallback...")
                    latex_code = self._generate_latex(sections, job_title, company)
                    pdf_bytes = self._compile_latex(latex_code)
                    if pdf_bytes:
                        method_used = "latex"
                except Exception as e:
                    latex_error = str(e)
                    print(f"LaTeX fallback also failed: {e}")

            if pdf_bytes:
                print(f"PDF generated successfully with {method_used}: {len(pdf_bytes)} bytes")
                return {
                    "success": True,
                    "pdf_base64": base64.b64encode(pdf_bytes).decode('utf-8'),
                    "size": len(pdf_bytes),
                    "method": method_used
                }

            # Both failed
            error_details = []
            if reportlab_error:
                error_details.append(f"reportlab: {reportlab_error}")
            if latex_error:
                error_details.append(f"LaTeX: {latex_error}")

            error_msg = "PDF generation failed. " + "; ".join(error_details) if error_details else "Both methods failed"
            return {"success": False, "error": error_msg, "pdf_base64": ""}

        except Exception as e:
            traceback.print_exc()
            return {"success": False, "error": f"PDF generation error: {str(e)}", "pdf_base64": ""}

    def _parse_cv_sections(self, cv_content, default_name, default_title):
        """Parse CV content into structured sections."""
        lines = cv_content.strip().split('\n')
        sections = {
            'name': default_name,
            'title': default_title,
            'profil': [],
            'competences': [],
            'experience': [],
            'formation': [],
            'interets': []
        }

        current_section = None

        # Section mapping (case-insensitive)
        section_map = {
            'nom': 'name', 'name': 'name',
            'titre': 'title', 'title': 'title',
            'profil': 'profil', 'profile': 'profil', 'summary': 'profil', 'résumé': 'profil',
            'competences': 'competences', 'compétences': 'competences', 'competences cles': 'competences',
            'compétences clés': 'competences', 'skills': 'competences',
            'experience': 'experience', 'expérience': 'experience', 'experience professionnelle': 'experience',
            'expérience professionnelle': 'experience', 'work experience': 'experience',
            'formation': 'formation', 'education': 'formation', 'études': 'formation',
            'interets': 'interets', 'intérêts': 'interets', 'interêts': 'interets',
            'centres d\'intérêt': 'interets', 'hobbies': 'interets', 'interests': 'interets'
        }

        for line in lines:
            line = line.strip()
            if not line:
                continue

            # Section tags [SECTION]
            if line.startswith('[') and line.endswith(']'):
                tag = line[1:-1].lower().strip()
                current_section = section_map.get(tag, tag if tag in sections else None)
                continue

            # Check for uppercase section headers (with or without colon)
            clean_line = line.replace(':', '').strip()
            if clean_line.isupper() or (len(clean_line) < 50 and clean_line.upper() == clean_line):
                lower_clean = clean_line.lower()
                if lower_clean in section_map:
                    current_section = section_map[lower_clean]
                    continue

            # Also check for "## Section" markdown headers
            if line.startswith('#'):
                header = line.lstrip('#').strip().lower()
                if header in section_map:
                    current_section = section_map[header]
                    continue

            # Add content to current section
            if current_section:
                if current_section == 'name':
                    sections['name'] = line
                    current_section = None  # Only take first line for name
                elif current_section == 'title':
                    sections['title'] = line
                    current_section = None  # Only take first line for title
                elif current_section in sections and isinstance(sections[current_section], list):
                    sections[current_section].append(line)

        return sections

    def _latex_escape(self, text):
        """Escape special LaTeX characters and convert markdown to LaTeX."""
        if not text:
            return ""
        # First convert **bold** to a placeholder (before escaping)
        import re
        bold_matches = re.findall(r'\*\*([^*]+)\*\*', text)
        for i, match in enumerate(bold_matches):
            text = text.replace(f'**{match}**', f'<<<BOLD{i}>>>', 1)

        # Escape LaTeX special characters
        replacements = [
            ('\\', '\\textbackslash{}'),
            ('&', '\\&'),
            ('%', '\\%'),
            ('$', '\\$'),
            ('#', '\\#'),
            ('_', '\\_'),
            ('{', '\\{'),
            ('}', '\\}'),
            ('~', '\\textasciitilde{}'),
            ('^', '\\textasciicircum{}'),
        ]
        for old, new in replacements:
            text = text.replace(old, new)

        # Restore bold with LaTeX formatting (escape the content too)
        for i, match in enumerate(bold_matches):
            escaped_match = match
            for old, new in replacements:
                escaped_match = escaped_match.replace(old, new)
            text = text.replace(f'<<<BOLD{i}>>>', f'\\textbf{{{escaped_match}}}')

        return text

    def _generate_latex(self, sections, job_title, company):
        """Generate ModernCV LaTeX code."""

        # Parse name
        name_parts = sections['name'].split()
        if len(name_parts) >= 2:
            firstname = self._latex_escape(name_parts[0])
            lastname = self._latex_escape(' '.join(name_parts[1:]))
        else:
            firstname = self._latex_escape(sections['name'])
            lastname = ""

        title = self._latex_escape(sections['title'] or job_title)

        # Build competences
        competences_latex = ""
        for line in sections['competences']:
            if '|' in line:
                parts = line.split('|', 1)
                label = self._latex_escape(parts[0].strip())
                value = self._latex_escape(parts[1].strip())
                competences_latex += f"\\cvitem{{{label}}}{{{value}}}\n"
            elif ':' in line:
                parts = line.split(':', 1)
                label = self._latex_escape(parts[0].strip())
                value = self._latex_escape(parts[1].strip())
                competences_latex += f"\\cvitem{{{label}}}{{{value}}}\n"

        # Build experience
        experience_latex = ""
        current_entry = None
        current_bullets = []

        for line in sections['experience']:
            if '|' in line and not line.startswith('-'):
                # Save previous entry
                if current_entry:
                    experience_latex += self._format_cventry(current_entry, current_bullets)
                    current_bullets = []
                # Parse: 2023-2025|Poste|Entreprise|Lieu
                parts = [p.strip() for p in line.split('|')]
                current_entry = {
                    'dates': self._latex_escape(parts[0]) if len(parts) > 0 else "",
                    'title': self._latex_escape(parts[1]) if len(parts) > 1 else "",
                    'company': self._latex_escape(parts[2]) if len(parts) > 2 else "",
                    'location': self._latex_escape(parts[3]) if len(parts) > 3 else "",
                }
            elif line.startswith('-') or line.startswith('•'):
                bullet = line.lstrip('-•* ').strip()
                current_bullets.append(self._latex_escape(bullet))

        if current_entry:
            experience_latex += self._format_cventry(current_entry, current_bullets)

        # Build formation
        formation_latex = ""
        for line in sections['formation']:
            if '|' in line:
                parts = [p.strip() for p in line.split('|')]
                dates = self._latex_escape(parts[0]) if len(parts) > 0 else ""
                diplome = self._latex_escape(parts[1]) if len(parts) > 1 else ""
                ecole = self._latex_escape(parts[2]) if len(parts) > 2 else ""
                detail = self._latex_escape(parts[3]) if len(parts) > 3 else ""
                formation_latex += f"\\cventry{{{dates}}}{{{diplome}}}{{{ecole}}}{{}}{{}}{{{detail}}}\n"

        # Build profil
        profil_text = ' '.join([self._latex_escape(p) for p in sections['profil']])

        # Build interets
        interets_text = ', '.join([self._latex_escape(i) for i in sections['interets']])

        # Generate full LaTeX document
        latex = f'''\\documentclass[a4paper,11pt]{{moderncv}}
\\moderncvstyle{{classic}}
\\moderncvcolor{{blue}}

\\usepackage[utf8]{{inputenc}}
\\usepackage[T1]{{fontenc}}
\\usepackage[scale=0.88]{{geometry}}
\\usepackage{{hyperref}}

\\name{{{firstname}}}{{{lastname}}}
\\title{{{title}}}

\\begin{{document}}
\\makecvtitle

\\section{{Profil}}
{profil_text}

\\section{{Compétences clés}}
{competences_latex}

\\section{{Expérience professionnelle}}
{experience_latex}

\\section{{Formation}}
{formation_latex}

\\section{{Centres d'intérêt}}
{interets_text}

\\end{{document}}
'''
        return latex

    def _format_cventry(self, entry, bullets):
        """Format a cventry with bullet points."""
        bullets_latex = ""
        if bullets:
            bullets_latex = "\\begin{itemize}\n"
            for b in bullets:
                bullets_latex += f"\\item {b}\n"
            bullets_latex += "\\end{itemize}"

        return f"""\\cventry{{{entry['dates']}}}{{{entry['title']}}}{{{entry['company']}}}{{{entry['location']}}}{{}}{{
{bullets_latex}}}

"""

    def _compile_latex(self, latex_code):
        """Compile LaTeX to PDF using pdflatex."""
        import shutil

        # Create temp directory
        temp_dir = tempfile.mkdtemp()
        tex_file = os.path.join(temp_dir, "cv.tex")
        pdf_file = os.path.join(temp_dir, "cv.pdf")

        try:
            # Write LaTeX file
            with open(tex_file, 'w', encoding='utf-8') as f:
                f.write(latex_code)

            # Compile with pdflatex (run twice for references)
            for _ in range(2):
                result = subprocess.run(
                    ['pdflatex', '-interaction=nonstopmode', '-output-directory', temp_dir, tex_file],
                    capture_output=True,
                    text=True,
                    timeout=60
                )

            # Check if PDF was created
            if os.path.exists(pdf_file):
                with open(pdf_file, 'rb') as f:
                    return f.read()
            else:
                print(f"LaTeX compilation failed. Log:\n{result.stdout}\n{result.stderr}")
                return None

        except subprocess.TimeoutExpired:
            print("LaTeX compilation timeout")
            return None
        except Exception as e:
            print(f"LaTeX compilation error: {e}")
            return None
        finally:
            # Cleanup
            shutil.rmtree(temp_dir, ignore_errors=True)

    def _generate_pdf_reportlab(self, sections, job_title, company):
        """Generate professional PDF using reportlab (ModernCV style)."""
        if not PDF_GENERATOR:
            raise Exception("reportlab not installed")

        from reportlab.lib.pagesizes import A4
        from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
        from reportlab.lib.units import cm, mm
        from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle, HRFlowable
        from reportlab.lib.enums import TA_LEFT, TA_CENTER, TA_RIGHT
        from reportlab.lib.colors import HexColor, Color
        from reportlab.platypus.flowables import Flowable
        from io import BytesIO

        # ModernCV Blue color scheme
        MAIN_COLOR = HexColor('#2E5090')  # ModernCV blue
        DARK_COLOR = HexColor('#1A1A1A')
        GRAY_COLOR = HexColor('#666666')
        LIGHT_GRAY = HexColor('#999999')

        buffer = BytesIO()
        doc = SimpleDocTemplate(
            buffer,
            pagesize=A4,
            rightMargin=1.2*cm,
            leftMargin=1.2*cm,
            topMargin=1.0*cm,
            bottomMargin=1.0*cm
        )

        styles = getSampleStyleSheet()

        # Custom styles - ModernCV inspired
        cv_styles = {
            'Name': ParagraphStyle(
                name='CVNameStyle',
                fontSize=24,
                textColor=DARK_COLOR,
                spaceAfter=8,
                alignment=TA_LEFT,
                fontName='Helvetica-Bold'
            ),
            'Title': ParagraphStyle(
                name='CVTitleStyle',
                fontSize=14,
                textColor=MAIN_COLOR,
                spaceAfter=8,
                alignment=TA_LEFT,
                fontName='Helvetica-Oblique'
            ),
            'Contact': ParagraphStyle(
                name='CVContactStyle',
                fontSize=9,
                textColor=GRAY_COLOR,
                spaceAfter=2,
                alignment=TA_LEFT
            ),
            'Section': ParagraphStyle(
                name='CVSectionStyle',
                fontSize=12,
                textColor=MAIN_COLOR,
                spaceBefore=14,
                spaceAfter=6,
                fontName='Helvetica-Bold'
            ),
            'EntryTitle': ParagraphStyle(
                name='CVEntryTitleStyle',
                fontSize=10,
                textColor=DARK_COLOR,
                spaceAfter=1,
                fontName='Helvetica-Bold'
            ),
            'EntryDetails': ParagraphStyle(
                name='CVEntryDetailsStyle',
                fontSize=10,
                textColor=GRAY_COLOR,
                spaceAfter=2,
                fontName='Helvetica-Oblique'
            ),
            'EntryDate': ParagraphStyle(
                name='CVEntryDateStyle',
                fontSize=9,
                textColor=MAIN_COLOR,
                alignment=TA_RIGHT,
                fontName='Helvetica'
            ),
            'Body': ParagraphStyle(
                name='CVBodyStyle',
                fontSize=10,
                textColor=DARK_COLOR,
                spaceAfter=4,
                leading=13
            ),
            'Bullet': ParagraphStyle(
                name='CVBulletStyle',
                fontSize=9,
                textColor=DARK_COLOR,
                leftIndent=12,
                spaceAfter=2,
                leading=12
            ),
            'SkillLabel': ParagraphStyle(
                name='CVSkillLabelStyle',
                fontSize=10,
                textColor=MAIN_COLOR,
                fontName='Helvetica-Bold'
            ),
            'SkillValue': ParagraphStyle(
                name='CVSkillValueStyle',
                fontSize=10,
                textColor=DARK_COLOR
            ),
        }

        story = []

        # === HEADER ===
        name = sections.get('name', 'Candidat') or 'Candidat'
        title = sections.get('title') or job_title or ''

        # Name
        story.append(Paragraph(self._escape(name), cv_styles['Name']))

        # Title
        if title:
            story.append(Paragraph(self._escape(title), cv_styles['Title']))

        # Horizontal line
        story.append(HRFlowable(width="100%", thickness=2, color=MAIN_COLOR, spaceAfter=10))

        has_content = False

        # === PROFIL ===
        profil = sections.get('profil', [])
        if profil:
            has_content = True
            story.append(Paragraph("Profil", cv_styles['Section']))
            story.append(HRFlowable(width="100%", thickness=0.5, color=MAIN_COLOR, spaceAfter=6))
            profil_text = ' '.join(profil)
            story.append(Paragraph(self._escape_with_bold(profil_text), cv_styles['Body']))

        # === COMPÉTENCES ===
        competences = sections.get('competences', [])
        if competences:
            has_content = True
            story.append(Paragraph("Compétences clés", cv_styles['Section']))
            story.append(HRFlowable(width="100%", thickness=0.5, color=MAIN_COLOR, spaceAfter=6))

            # Create a table for skills
            skill_data = []
            for line in competences:
                if '|' in line:
                    parts = line.split('|', 1)
                    label = parts[0].strip()
                    value = parts[1].strip()
                    skill_data.append([
                        Paragraph(f"<b>{self._escape(label)}</b>", cv_styles['SkillLabel']),
                        Paragraph(self._escape(value), cv_styles['SkillValue'])
                    ])
                elif ':' in line:
                    parts = line.split(':', 1)
                    label = parts[0].strip()
                    value = parts[1].strip()
                    skill_data.append([
                        Paragraph(f"<b>{self._escape(label)}</b>", cv_styles['SkillLabel']),
                        Paragraph(self._escape(value), cv_styles['SkillValue'])
                    ])

            if skill_data:
                skill_table = Table(skill_data, colWidths=[4*cm, 13*cm])
                skill_table.setStyle(TableStyle([
                    ('VALIGN', (0, 0), (-1, -1), 'TOP'),
                    ('TOPPADDING', (0, 0), (-1, -1), 2),
                    ('BOTTOMPADDING', (0, 0), (-1, -1), 2),
                ]))
                story.append(skill_table)

        # === EXPÉRIENCE ===
        experience = sections.get('experience', [])
        if experience:
            has_content = True
            story.append(Paragraph("Expérience professionnelle", cv_styles['Section']))
            story.append(HRFlowable(width="100%", thickness=0.5, color=MAIN_COLOR, spaceAfter=6))

            current_entry = None
            current_bullets = []

            for line in experience:
                if '|' in line and not line.startswith('-'):
                    # Output previous entry
                    if current_entry:
                        self._add_experience_entry(story, current_entry, current_bullets, cv_styles)
                        current_bullets = []

                    parts = [p.strip() for p in line.split('|')]
                    current_entry = {
                        'dates': parts[0] if len(parts) > 0 else "",
                        'title': parts[1] if len(parts) > 1 else "",
                        'company': parts[2] if len(parts) > 2 else "",
                        'location': parts[3] if len(parts) > 3 else "",
                    }
                elif line.startswith('-') or line.startswith('•'):
                    bullet = line.lstrip('-•* ').strip()
                    current_bullets.append(bullet)

            # Output last entry
            if current_entry:
                self._add_experience_entry(story, current_entry, current_bullets, cv_styles)

        # === FORMATION ===
        formation = sections.get('formation', [])
        if formation:
            has_content = True
            story.append(Paragraph("Formation", cv_styles['Section']))
            story.append(HRFlowable(width="100%", thickness=0.5, color=MAIN_COLOR, spaceAfter=6))

            for line in formation:
                if '|' in line:
                    parts = [p.strip() for p in line.split('|')]
                    dates = parts[0] if len(parts) > 0 else ""
                    diplome = parts[1] if len(parts) > 1 else ""
                    ecole = parts[2] if len(parts) > 2 else ""
                    details = parts[3] if len(parts) > 3 else ""

                    # Create entry table
                    entry_data = [[
                        Paragraph(f"<b>{self._escape(diplome)}</b>", cv_styles['EntryTitle']),
                        Paragraph(self._escape(dates), cv_styles['EntryDate'])
                    ]]
                    entry_table = Table(entry_data, colWidths=[13*cm, 4*cm])
                    entry_table.setStyle(TableStyle([
                        ('VALIGN', (0, 0), (-1, -1), 'TOP'),
                        ('TOPPADDING', (0, 0), (-1, -1), 0),
                        ('BOTTOMPADDING', (0, 0), (-1, -1), 0),
                    ]))
                    story.append(entry_table)

                    if ecole:
                        story.append(Paragraph(self._escape(ecole), cv_styles['EntryDetails']))
                    if details:
                        story.append(Paragraph(self._escape(details), cv_styles['Body']))
                    story.append(Spacer(1, 4))

        # === CENTRES D'INTÉRÊT ===
        interets = sections.get('interets', [])
        if interets:
            has_content = True
            story.append(Paragraph("Centres d'intérêt", cv_styles['Section']))
            story.append(HRFlowable(width="100%", thickness=0.5, color=MAIN_COLOR, spaceAfter=6))
            interets_text = ', '.join(interets)
            story.append(Paragraph(self._escape(interets_text), cv_styles['Body']))

        # Fallback if no content
        if not has_content:
            print("Warning: No structured sections found")
            story.append(Paragraph("Contenu", cv_styles['Section']))
            story.append(Paragraph("Le CV n'a pas pu être structuré automatiquement.", cv_styles['Body']))

        doc.build(story)
        pdf_data = buffer.getvalue()

        if len(pdf_data) < 1000:
            raise Exception("Generated PDF is too small, likely empty")

        return pdf_data

    def _add_experience_entry(self, story, entry, bullets, cv_styles):
        """Add an experience entry to the story."""
        from reportlab.platypus import Paragraph, Spacer, Table, TableStyle
        from reportlab.lib.units import cm

        # Title + Date row
        entry_data = [[
            Paragraph(f"<b>{self._escape(entry['title'])}</b>", cv_styles['EntryTitle']),
            Paragraph(self._escape(entry['dates']), cv_styles['EntryDate'])
        ]]
        entry_table = Table(entry_data, colWidths=[13*cm, 4*cm])
        entry_table.setStyle(TableStyle([
            ('VALIGN', (0, 0), (-1, -1), 'TOP'),
            ('TOPPADDING', (0, 0), (-1, -1), 0),
            ('BOTTOMPADDING', (0, 0), (-1, -1), 0),
        ]))
        story.append(entry_table)

        # Company + Location
        company_loc = entry['company']
        if entry['location']:
            company_loc += f" — {entry['location']}"
        if company_loc:
            story.append(Paragraph(self._escape(company_loc), cv_styles['EntryDetails']))

        # Bullets
        for bullet in bullets:
            story.append(Paragraph(f"• {self._escape_with_bold(bullet)}", cv_styles['Bullet']))

        story.append(Spacer(1, 6))

    def _escape(self, text):
        """Escape HTML special characters."""
        if not text:
            return ""
        return text.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;')

    def _escape_with_bold(self, text):
        """Escape HTML and convert **bold** markdown to <b> tags for reportlab."""
        if not text:
            return ""
        import re
        # First escape HTML
        text = self._escape(text)
        # Then convert **bold** to <b>bold</b> (after escaping, so tags won't be escaped)
        text = re.sub(r'\*\*([^*]+)\*\*', r'<b>\1</b>', text)
        return text

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

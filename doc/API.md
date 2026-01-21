# API Reference - Claude HTTP Server

Le serveur Claude expose une API REST pour interagir avec Claude Code CLI. Il sert d'intermédiaire entre le bot Discord (Rust) et le CLI `claude`.

## Configuration

| Variable | Défaut | Description |
|----------|--------|-------------|
| `PORT` | `8080` | Port d'écoute HTTP |
| `CLAUDE_API_URL` | `http://claudecode:8080` | URL côté client Rust |

## Base URL

```
http://claudecode:8080  (Docker)
http://localhost:8080   (Local)
```

---

## Endpoints

### GET /health

Health check pour vérifier que le serveur est opérationnel.

**Requête:**
```bash
curl -X GET http://localhost:8080/health
```

**Réponse 200:**
```json
{
  "status": "ok",
  "service": "claude-server"
}
```

---

### POST /prompt

Envoie un prompt générique à Claude.

**Requête:**
```bash
curl -X POST http://localhost:8080/prompt \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "Explique-moi le machine learning en 3 phrases."
  }'
```

**Paramètres:**

| Champ | Type | Requis | Description |
|-------|------|--------|-------------|
| `prompt` | string | Oui | Le prompt à envoyer à Claude |

**Réponse 200:**
```json
{
  "response": "Le machine learning est une branche de l'intelligence artificielle..."
}
```

**Erreurs:**

| Code | Message | Cause |
|------|---------|-------|
| 400 | `Missing 'prompt' field` | Champ prompt absent |
| 500 | `Claude timeout after 120s` | Timeout dépassé |
| 500 | `Claude error: ...` | Erreur CLI Claude |

---

### POST /synthesize

Synthétise une offre d'emploi en extrayant les informations clés.

**Requête:**
```bash
curl -X POST http://localhost:8080/synthesize \
  -H "Content-Type: application/json" \
  -d '{
    "job_description": "Développeur Full Stack Senior chez TechCorp. CDI basé à Paris. Nous recherchons un profil avec 5 ans d'\''expérience..."
  }'
```

**Paramètres:**

| Champ | Type | Requis | Description |
|-------|------|--------|-------------|
| `job_description` | string | Oui | Texte complet de l'offre d'emploi |

**Réponse 200:**
```json
{
  "title": "Développeur Full Stack Senior",
  "company": "TechCorp",
  "location": "Paris",
  "contract_type": "CDI",
  "key_requirements": [
    "5 ans d'expérience",
    "Python",
    "React",
    "PostgreSQL"
  ],
  "responsibilities": [
    "Développer de nouvelles fonctionnalités",
    "Maintenir le code existant"
  ],
  "benefits": [
    "Télétravail partiel",
    "RTT"
  ],
  "salary_range": "55-70k€",
  "summary": "TechCorp recherche un développeur Full Stack expérimenté pour renforcer son équipe technique."
}
```

**Champs de réponse:**

| Champ | Type | Description |
|-------|------|-------------|
| `title` | string | Intitulé du poste |
| `company` | string | Nom de l'entreprise |
| `location` | string | Lieu de travail |
| `contract_type` | string | Type de contrat (CDI, CDD, etc.) |
| `key_requirements` | string[] | Compétences requises |
| `responsibilities` | string[] | Responsabilités du poste |
| `benefits` | string[] | Avantages proposés |
| `salary_range` | string\|null | Fourchette salariale si mentionnée |
| `summary` | string | Résumé en 2-3 phrases |

---

### POST /match-skills

Compare les compétences d'un CV avec les exigences d'une offre.

**Requête:**
```bash
curl -X POST http://localhost:8080/match-skills \
  -H "Content-Type: application/json" \
  -d '{
    "job_description": "Développeur Python Senior, 5 ans exp requis...",
    "cv_content": "Jean Dupont - 7 ans expérience Python, Django..."
  }'
```

**Paramètres:**

| Champ | Type | Requis | Description |
|-------|------|--------|-------------|
| `job_description` | string | Oui | Description de l'offre |
| `cv_content` | string | Non* | Contenu textuel du CV |

*Si absent, défaut à "CV non fourni"

**Réponse 200:**
```json
{
  "match_score": 78,
  "matched_skills": [
    {
      "skill": "Python",
      "cv_level": "7 ans",
      "required": "5 ans",
      "match": true
    },
    {
      "skill": "Django",
      "cv_level": "4 ans",
      "required": "requis",
      "match": true
    }
  ],
  "missing_skills": [
    {
      "skill": "Kubernetes",
      "importance": "nice-to-have"
    }
  ],
  "highlights": [
    "Expérience Python supérieure aux attentes",
    "Solide background Django"
  ],
  "recommendations": [
    "Mettre en avant les 7 ans d'expérience Python",
    "Mentionner des projets Django concrets"
  ]
}
```

**Champs de réponse:**

| Champ | Type | Description |
|-------|------|-------------|
| `match_score` | int (0-100) | Score de compatibilité |
| `matched_skills` | MatchedSkill[] | Compétences correspondantes |
| `missing_skills` | MissingSkill[] | Compétences manquantes |
| `highlights` | string[] | Points forts à valoriser |
| `recommendations` | string[] | Conseils d'amélioration |

**Type MatchedSkill:**
```typescript
{
  skill: string;      // Nom de la compétence
  cv_level: string;   // Niveau dans le CV
  required: string;   // Niveau requis
  match: boolean;     // Correspondance satisfaite
}
```

**Type MissingSkill:**
```typescript
{
  skill: string;      // Nom de la compétence
  importance: string; // "required" | "nice-to-have"
}
```

---

### POST /salary-analysis

Analyse le positionnement salarial pour un poste.

**Requête:**
```bash
curl -X POST http://localhost:8080/salary-analysis \
  -H "Content-Type: application/json" \
  -d '{
    "job_description": "Développeur Senior Python, CDI Paris",
    "location": "Paris, France"
  }'
```

**Paramètres:**

| Champ | Type | Requis | Description |
|-------|------|--------|-------------|
| `job_description` | string | Oui | Description de l'offre |
| `location` | string | Non | Localisation (défaut: "France") |

**Réponse 200:**
```json
{
  "offered_min": 55000,
  "offered_max": 70000,
  "market_low": 50000,
  "market_median": 60000,
  "market_high": 75000,
  "currency": "EUR",
  "analysis": "Le salaire proposé se situe dans la fourchette haute du marché parisien pour ce type de profil.",
  "negotiation_tips": [
    "Valorisez votre expérience supérieure à 5 ans",
    "Mentionnez vos certifications techniques",
    "Négociez les avantages annexes (télétravail, RTT)"
  ]
}
```

**Champs de réponse:**

| Champ | Type | Description |
|-------|------|-------------|
| `offered_min` | int\|null | Salaire minimum proposé (annuel brut) |
| `offered_max` | int\|null | Salaire maximum proposé |
| `market_low` | int | Salaire bas du marché |
| `market_median` | int | Salaire médian du marché |
| `market_high` | int | Salaire haut du marché |
| `currency` | string | Devise (EUR, USD, etc.) |
| `analysis` | string | Analyse textuelle du positionnement |
| `negotiation_tips` | string[] | Conseils de négociation |

---

### POST /generate-cv

Génère un CV adapté au format LaTeX.

**Requête:**
```bash
curl -X POST http://localhost:8080/generate-cv \
  -H "Content-Type: application/json" \
  -d '{
    "cv_content": "Jean Dupont\nDéveloppeur Python\n7 ans exp...",
    "job_title": "Développeur Senior",
    "company": "TechCorp",
    "requirements": ["Python", "Django", "PostgreSQL"],
    "highlights": ["7 ans Python", "Expert Django"]
  }'
```

**Paramètres:**

| Champ | Type | Requis | Description |
|-------|------|--------|-------------|
| `cv_content` | string | Oui | Contenu du CV original |
| `job_title` | string | Non | Titre du poste visé |
| `company` | string | Non | Nom de l'entreprise |
| `requirements` | string[] | Non | Compétences requises |
| `highlights` | string[] | Non | Points forts identifiés |

**Réponse 200:**
```json
{
  "latex_content": "\\documentclass[11pt,a4paper]{article}\n\\usepackage[utf8]{inputenc}\n...",
  "adaptations": [
    "Mise en avant de l'expérience Python (7 ans)",
    "Section Django déplacée en première position",
    "Ajout de mots-clés correspondant à l'offre"
  ],
  "summary": "CV adapté pour le poste de Développeur Senior chez TechCorp, mettant en avant l'expertise Python et Django."
}
```

**Champs de réponse:**

| Champ | Type | Description |
|-------|------|-------------|
| `latex_content` | string | Code source LaTeX du CV |
| `adaptations` | string[] | Liste des modifications apportées |
| `summary` | string | Résumé des adaptations |

---

## Gestion des erreurs

### Format d'erreur

Toutes les erreurs retournent un JSON avec le champ `error`:

```json
{
  "error": "Description de l'erreur"
}
```

### Codes HTTP

| Code | Signification | Causes possibles |
|------|---------------|------------------|
| 200 | Succès | Requête traitée |
| 400 | Bad Request | JSON invalide, champ manquant |
| 404 | Not Found | Endpoint inexistant |
| 500 | Internal Error | Erreur Claude, timeout |

### Fallback raw_response

Si Claude retourne une réponse non-JSON, le serveur encapsule la réponse brute:

```json
{
  "raw_response": "Texte brut retourné par Claude..."
}
```

**Note:** Côté client Rust, `raw_response` est traité comme une erreur.

---

## Limites et timeouts

| Paramètre | Valeur |
|-----------|--------|
| Timeout par requête | 120 secondes |
| Taille max du body | Illimitée (Python) |
| Requêtes concurrentes | 1 (sérialisé par le CLI) |

---

## Exemple d'intégration (Rust)

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SynthesizeRequest {
    job_description: String,
}

#[derive(Deserialize)]
struct JobSynthesis {
    title: String,
    company: String,
    location: String,
    // ...
}

async fn synthesize_job(description: &str) -> Result<JobSynthesis, reqwest::Error> {
    let client = Client::new();
    let response = client
        .post("http://claudecode:8080/synthesize")
        .json(&SynthesizeRequest {
            job_description: description.to_string(),
        })
        .send()
        .await?
        .json::<JobSynthesis>()
        .await?;

    Ok(response)
}
```

---

## Logs

Le serveur log toutes les requêtes dans stdout:

```
[21/Jan/2025 12:34:56] "POST /synthesize HTTP/1.1" 200 -
[21/Jan/2025 12:35:10] "POST /match-skills HTTP/1.1" 200 -
```

Pour activer les logs debug côté client Rust:
```bash
export RUST_LOG=debug
```

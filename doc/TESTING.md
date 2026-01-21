# Protocole de Test

Ce document décrit les procédures de test pour valider le bon fonctionnement du bot Automate-CV-DiscordBot.

---

## T1. Tests Unitaires Rust

### T1.1 Exécution des tests

```bash
cd /workspace/rust/Automate-CV-discordBot

# Tous les tests
cargo test

# Tests avec output
cargo test -- --nocapture

# Tests d'un module spécifique
cargo test db:: -- --nocapture
cargo test services:: -- --nocapture

# Test unique
cargo test test_init_database -- --nocapture
```

### T1.2 Tests de la base de données

**Fichier:** `src/db/init.rs`

| Test | Description | Critère de succès |
|------|-------------|-------------------|
| `test_init_database` | Création des tables | Tables users, base_cvs, job_applications, application_status_history existent |

**Ajouter des tests (TODO):**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upsert_user() {
        let db = Database::in_memory().unwrap();
        db.upsert_user(123456789, "testuser").unwrap();
        let user = db.get_user(123456789).unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().username, "testuser");
    }

    #[test]
    fn test_save_and_get_cv() {
        let db = Database::in_memory().unwrap();
        db.upsert_user(123, "user").unwrap();
        let cv_id = db.save_cv(123, "test.pdf", "CV.pdf", "/path", 1024, Some("application/pdf")).unwrap();
        let cv = db.get_active_cv(123).unwrap();
        assert!(cv.is_some());
        assert_eq!(cv.unwrap().id, cv_id);
    }

    #[test]
    fn test_delete_cv() {
        let db = Database::in_memory().unwrap();
        db.upsert_user(123, "user").unwrap();
        db.save_cv(123, "test.pdf", "CV.pdf", "/path", 1024, None).unwrap();
        let deleted = db.delete_active_cv(123).unwrap();
        assert!(deleted);
        let cv = db.get_active_cv(123).unwrap();
        assert!(cv.is_none());
    }
}
```

---

## T2. Tests d'intégration API Claude

### T2.1 Health Check

```bash
curl -X GET http://localhost:8080/health
```

**Réponse attendue:**
```json
{"status": "ok", "service": "claude-server"}
```

### T2.2 Endpoint /prompt

```bash
curl -X POST http://localhost:8080/prompt \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Réponds uniquement: OK"}'
```

**Réponse attendue:**
```json
{"response": "OK"}
```

### T2.3 Endpoint /synthesize

```bash
curl -X POST http://localhost:8080/synthesize \
  -H "Content-Type: application/json" \
  -d '{
    "job_description": "Développeur Full Stack Senior chez TechCorp. CDI basé à Paris. Nous recherchons un profil avec 5 ans d'\''expérience en Python, React et PostgreSQL. Salaire: 55-70k€."
  }'
```

**Réponse attendue (structure):**
```json
{
  "title": "Développeur Full Stack Senior",
  "company": "TechCorp",
  "location": "Paris",
  "contract_type": "CDI",
  "key_requirements": ["Python", "React", "PostgreSQL", "5 ans d'expérience"],
  "responsibilities": [...],
  "benefits": [...],
  "salary_range": "55-70k€",
  "summary": "..."
}
```

**Critères de validation:**
- [ ] Statut HTTP 200
- [ ] JSON valide
- [ ] Champs `title`, `company`, `location` présents
- [ ] `key_requirements` est un tableau non vide

### T2.4 Endpoint /match-skills

```bash
curl -X POST http://localhost:8080/match-skills \
  -H "Content-Type: application/json" \
  -d '{
    "job_description": "Développeur Python Senior, 5 ans exp requis. Compétences: Django, PostgreSQL, Docker.",
    "cv_content": "Jean Dupont - Développeur Python\n- 7 ans expérience Python\n- Django (4 ans)\n- Flask (3 ans)\n- PostgreSQL (2 ans)\n- Docker basique"
  }'
```

**Réponse attendue (structure):**
```json
{
  "match_score": 75,
  "matched_skills": [
    {"skill": "Python", "cv_level": "7 ans", "required": "5 ans", "match": true},
    {"skill": "Django", "cv_level": "4 ans", "required": "requis", "match": true}
  ],
  "missing_skills": [
    {"skill": "Docker avancé", "importance": "nice-to-have"}
  ],
  "highlights": ["Expérience Python supérieure aux attentes"],
  "recommendations": ["Mettre en avant l'expérience Django"]
}
```

**Critères de validation:**
- [ ] `match_score` entre 0 et 100
- [ ] `matched_skills` contient les compétences communes
- [ ] `missing_skills` identifie les lacunes

### T2.5 Endpoint /salary-analysis

```bash
curl -X POST http://localhost:8080/salary-analysis \
  -H "Content-Type: application/json" \
  -d '{
    "job_description": "Développeur Senior Python, CDI Paris, 5 ans exp",
    "location": "Paris, France"
  }'
```

**Réponse attendue (structure):**
```json
{
  "offered_min": null,
  "offered_max": null,
  "market_low": 50000,
  "market_median": 60000,
  "market_high": 75000,
  "currency": "EUR",
  "analysis": "Le marché parisien pour ce profil...",
  "negotiation_tips": ["Valorisez votre expérience...", "..."]
}
```

**Critères de validation:**
- [ ] `market_low` < `market_median` < `market_high`
- [ ] `currency` est "EUR" pour la France
- [ ] `analysis` est une chaîne non vide

### T2.6 Endpoint /generate-cv

```bash
curl -X POST http://localhost:8080/generate-cv \
  -H "Content-Type: application/json" \
  -d '{
    "cv_content": "Jean Dupont\nDéveloppeur Python\n7 ans exp\nDjango, Flask, PostgreSQL",
    "job_title": "Développeur Senior",
    "company": "TechCorp",
    "requirements": ["Python", "Django", "PostgreSQL"],
    "highlights": ["7 ans Python", "Expert Django"]
  }'
```

**Réponse attendue (structure):**
```json
{
  "latex_content": "\\documentclass{article}...",
  "adaptations": ["Mise en avant de l'expérience Django", "..."],
  "summary": "CV adapté pour le poste de Développeur Senior chez TechCorp"
}
```

**Critères de validation:**
- [ ] `latex_content` contient du code LaTeX valide
- [ ] `adaptations` liste les modifications apportées
- [ ] `summary` résume l'adaptation

---

## T3. Tests Fonctionnels Discord

### T3.1 Préparation

1. Inviter le bot sur un serveur de test
2. S'assurer d'avoir les permissions administrateur
3. Préparer un fichier CV de test (PDF, <10 Mo)

### T3.2 Tests des commandes de base

| ID | Commande | Action | Résultat attendu | Statut |
|----|----------|--------|------------------|--------|
| T3.2.1 | `/help` | Exécuter | Liste de toutes les commandes | [ ] |
| T3.2.2 | `/listmycvs` | Exécuter (nouveau user) | "Aucun CV enregistré" | [ ] |
| T3.2.3 | `/status` | Exécuter (nouveau user) | "Aucune candidature" | [ ] |
| T3.2.4 | `/mystats` | Exécuter (nouveau user) | Statistiques à zéro | [ ] |

### T3.3 Tests du workflow CV

| ID | Commande | Action | Résultat attendu | Statut |
|----|----------|--------|------------------|--------|
| T3.3.1 | `/sendcv` | Upload PDF valide | "CV enregistré avec succès" | [ ] |
| T3.3.2 | `/sendcv` | Upload .exe | "Type de fichier non supporté" | [ ] |
| T3.3.3 | `/sendcv` | Upload fichier >25Mo | Erreur Discord | [ ] |
| T3.3.4 | `/listmycvs` | Après upload | CV visible, statut "Actif" | [ ] |
| T3.3.5 | `/sendcv` | Deuxième CV | Nouveau CV actif, ancien inactif | [ ] |
| T3.3.6 | `/deletecv` | Supprimer CV actif | "CV supprimé" | [ ] |
| T3.3.7 | `/deletecv` | Sans CV actif | "Aucun CV actif trouvé" | [ ] |

### T3.4 Tests du workflow candidature

| ID | Commande | Action | Résultat attendu | Statut |
|----|----------|--------|------------------|--------|
| T3.4.1 | `/applyjob` | Sans CV uploadé | Analyse partielle + "Uploadez votre CV" | [ ] |
| T3.4.2 | `/applyjob` | Avec CV uploadé | 4 embeds: Synthèse, Skills, Salaire, CV | [ ] |

**Détail des embeds attendus pour T3.4.2:**

1. **Embed Synthèse (Vert)**
   - [ ] Titre: "SYNTHÈSE DE L'OFFRE"
   - [ ] Champs: Entreprise, Poste, Lieu, Contrat
   - [ ] Compétences clés listées
   - [ ] Résumé présent

2. **Embed Compétences (Jaune)**
   - [ ] Titre: "ANALYSE DE COMPATIBILITÉ"
   - [ ] Score de matching avec barre de progression
   - [ ] Compétences matchées avec niveaux
   - [ ] Compétences manquantes identifiées

3. **Embed Salaire (Orange)**
   - [ ] Titre: "ANALYSE SALARIALE"
   - [ ] Fourchettes marché (bas/médian/haut)
   - [ ] Analyse textuelle
   - [ ] Conseils de négociation

4. **Embed CV (Bleu)**
   - [ ] Titre: "CV PERSONNALISÉ GÉNÉRÉ"
   - [ ] Résumé des adaptations
   - [ ] Liste des modifications

### T3.5 Tests des commandes admin

| ID | Commande | Utilisateur | Résultat attendu | Statut |
|----|----------|-------------|------------------|--------|
| T3.5.1 | `/listcvs` | Non-admin | "Permission denied" | [ ] |
| T3.5.2 | `/listcvs` | Admin | Liste de tous les CVs | [ ] |
| T3.5.3 | `/getcv @user` | Admin | CV de l'utilisateur ciblé | [ ] |
| T3.5.4 | `/clearallcvs` | Admin | Demande de confirmation | [ ] |

---

## T4. Tests de robustesse

### T4.1 Timeout Claude

```bash
# Simuler un prompt très long qui timeout
curl -X POST http://localhost:8080/prompt \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Écris un essai de 50000 mots sur la philosophie."}'
```

**Résultat attendu:** Erreur timeout après 120s

### T4.2 Redémarrage des services

```bash
# 1. Arrêter le serveur Claude
docker stop claudecode

# 2. Exécuter /applyjob sur Discord
# Résultat attendu: Message d'erreur gracieux

# 3. Redémarrer
docker start claudecode

# 4. Réessayer /applyjob
# Résultat attendu: Fonctionnement normal
```

### T4.3 Persistance des données

```bash
# 1. Uploader un CV via /sendcv
# 2. Arrêter et supprimer les conteneurs
docker compose down

# 3. Relancer
docker compose up -d

# 4. Vérifier avec /listmycvs
# Résultat attendu: CV toujours présent
```

### T4.4 Entrées malformées

| Test | Entrée | Résultat attendu |
|------|--------|------------------|
| JSON invalide | `curl -X POST -d 'not json'` | HTTP 400 |
| Prompt vide | `{"prompt": ""}` | Erreur "Missing prompt" |
| Champs manquants | `{"wrong_field": "..."}` | Erreur explicative |

---

## T5. Checklist de validation finale

### Fonctionnalités core

- [ ] Upload CV (PDF, DOC, TXT)
- [ ] Extraction de texte via Claude
- [ ] Stockage SQLite
- [ ] Synthèse d'offre d'emploi
- [ ] Matching compétences CV/Offre
- [ ] Analyse salariale
- [ ] Génération CV adapté

### Infrastructure

- [ ] Build Docker reproductible
- [ ] Volumes persistants
- [ ] Réseau inter-conteneurs fonctionnel
- [ ] Health checks OK
- [ ] Logs accessibles et informatifs

### Sécurité

- [ ] Token Discord non exposé
- [ ] Permissions admin vérifiées côté serveur
- [ ] Types de fichiers validés
- [ ] Isolation des données par utilisateur

### Performance

- [ ] Commandes répondent en <3s (avant defer)
- [ ] Analyse complète en <2min
- [ ] Pas de fuite mémoire sur longue durée

---

## Rapport de test

### Template

```markdown
# Rapport de Test - [DATE]

## Environnement
- Version du bot: [commit hash]
- Version Docker: [version]
- Serveur Discord: [nom]

## Résultats

### Tests unitaires
- Total: X
- Passés: X
- Échoués: X

### Tests API
- /health: [OK/FAIL]
- /synthesize: [OK/FAIL]
- /match-skills: [OK/FAIL]
- /salary-analysis: [OK/FAIL]
- /generate-cv: [OK/FAIL]

### Tests Discord
- Commandes de base: X/4
- Workflow CV: X/7
- Workflow candidature: X/2
- Admin: X/4

### Problèmes identifiés
1. [Description du problème]
   - Sévérité: [Critique/Majeur/Mineur]
   - Reproductible: [Oui/Non]

## Conclusion
[Validation / Refus avec conditions]
```

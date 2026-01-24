# Architecture du Projet Automate-CV-DiscordBot

## Vue d'ensemble

Ce projet est un bot Discord écrit en Rust qui automatise la personnalisation de CV en fonction des offres d'emploi. Il utilise Claude (via un serveur HTTP intermédiaire) pour l'analyse IA.

> **Note:** Pour les diagrammes détaillés en Mermaid (architecture, séquences, états, classes, ERD, flowchart, mindmap, user journey), voir [DIAGRAMS.md](DIAGRAMS.md).

## Diagramme d'architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        DISCORD                                   │
│                     (Utilisateurs)                               │
└────────────────────────┬────────────────────────────────────────┘
                         │ Slash Commands
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│              DISCORD BOT (Rust/Serenity)                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐        │
│  │ commands/│  │ services/│  │   db/    │  │  main.rs │        │
│  │ cv.rs    │  │ claude_  │  │ SQLite   │  │ Handler  │        │
│  │ jobs.rs  │  │ client.rs│  │ wrapper  │  │          │        │
│  │ admin.rs │  │          │  │          │  │          │        │
│  └──────────┘  └────┬─────┘  └────┬─────┘  └──────────┘        │
└──────────────────────┼────────────┼─────────────────────────────┘
                       │ HTTP       │ File I/O
                       ▼            ▼
┌──────────────────────────┐   ┌──────────────────────────────────┐
│   CLAUDE SERVER (Python) │   │        DATA STORAGE              │
│   - /synthesize          │   │   ┌──────────┐  ┌─────────────┐  │
│   - /match-skills        │   │   │ dbLookout│  │ data/cvs/   │  │
│   - /salary-analysis     │   │   │ bot.db   │  │ fichiers CV │  │
│   - /generate-cv         │   │   └──────────┘  └─────────────┘  │
│   - /extract-pdf         │                                       │
│   - /generate-pdf        │                                       │
└────────────┬─────────────┘   └──────────────────────────────────┘
             │ subprocess
             ▼
┌──────────────────────────┐
│    CLAUDE CODE CLI       │
│    (claude -p "prompt")  │
└──────────────────────────┘
```

## Structure des fichiers

```
Automate-CV-discordBot/
├── Cargo.toml              # Dépendances Rust
├── Cargo.lock              # Versions verrouillées
├── Dockerfile              # Image Docker du bot
├── docker-compose.yml      # Orchestration des services
├── .env                    # Variables d'environnement (non versionné)
├── .env.example            # Template des variables
├── README.MD               # Documentation principale
│
├── src/
│   ├── main.rs             # Point d'entrée, initialisation
│   │
│   ├── commands/           # Commandes Discord slash
│   │   ├── mod.rs          # Trait SlashCommand + CommandRegistry
│   │   ├── cv.rs           # /sendcv, /deletecv, /listmycvs
│   │   ├── jobs.rs         # /applyjob, /status, /updatestatus, /mystats
│   │   ├── generation.rs   # /synthesizeoffer, /generateresume, etc.
│   │   ├── admin.rs        # /listcvs, /getcv, /clearallcvs
│   │   └── help.rs         # /help
│   │
│   ├── db/                 # Couche base de données
│   │   ├── mod.rs          # Wrapper Database thread-safe
│   │   ├── init.rs         # Création des tables SQLite
│   │   └── utilities.rs    # Opérations CRUD
│   │
│   └── services/           # Services externes
│       ├── mod.rs          # Exports
│       └── claude_client.rs # Client HTTP pour Claude Server
│
├── claude-server/          # Serveur HTTP Python pour Claude
│   ├── Dockerfile          # Image basée sur claudeclode
│   └── server.py           # Wrapper HTTP pour claude CLI
│
├── dbLookout/              # Dossier de la base de données
│   └── bot.db              # Fichier SQLite (généré)
│
├── data/                   # Données runtime
│   └── cvs/                # CVs uploadés (généré)
│
└── doc/                    # Documentation
    ├── ARCHITECTURE.md     # Ce fichier
    ├── DEPLOYMENT.md       # Guide de déploiement
    ├── TESTING.md          # Protocole de test
    ├── API.md              # Documentation API Claude Server
    └── COMMANDS.md         # Référence des commandes Discord
```

## Composants principaux

### 1. Bot Discord (Rust)

**Framework:** Serenity 0.12

**Responsabilités:**
- Gestion des événements Discord (ready, interaction_create)
- Enregistrement des slash commands
- Dispatch des commandes vers les handlers appropriés
- Injection de dépendances via TypeMap

**Fichier clé:** `src/main.rs`

```rust
// Pattern d'injection de dépendances
impl TypeMapKey for Database { ... }
impl TypeMapKey for ClaudeClientKey { ... }
```

### 2. Système de commandes

**Pattern:** Trait `SlashCommand` avec implémentation par commande

```rust
#[async_trait]
pub trait SlashCommand: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn register(&self) -> CreateCommand;
    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction)
        -> Result<(), CommandError>;
}
```

**CommandRegistry:** Centralise l'enregistrement et le dispatch

### 3. Base de données SQLite

**Wrapper thread-safe:** `Arc<Mutex<Connection>>`

**Tables:**

| Table | Clé primaire | Description |
|-------|--------------|-------------|
| `users` | Discord user ID | Profils utilisateurs |
| `base_cvs` | Auto-increment | CVs uploadés |
| `job_applications` | Auto-increment | Candidatures avec analyses |
| `application_status_history` | Auto-increment | Historique des statuts |

**Relations:**
- `base_cvs.user_id` → `users.id`
- `job_applications.user_id` → `users.id`
- `job_applications.base_cv_id` → `base_cvs.id`
- `application_status_history.application_id` → `job_applications.id`

### 4. Client Claude HTTP

**Fichier:** `src/services/claude_client.rs`

**Configuration:**
```rust
let base_url = env::var("CLAUDE_API_URL")
    .unwrap_or_else(|_| "http://claudecode:8080".to_string());
```

**Timeout:** 120 secondes par requête

**Structures de données:**
- `JobSynthesis` - Résultat de synthèse d'offre
- `SkillsMatch` - Résultat de matching compétences
- `SalaryAnalysis` - Analyse salariale
- `GeneratedCv` - CV généré en LaTeX

### 5. Serveur Claude (Python)

**Fichier:** `claude-server/server.py`

**Rôle:** Wrapper HTTP autour de `claude -p`

**Fonctionnement:**
1. Reçoit une requête HTTP POST
2. Construit un prompt structuré
3. Exécute `subprocess.run(["claude", "-p", prompt])`
4. Parse la réponse JSON
5. Retourne le résultat

## Flux de données

### Workflow /applyjob

```
1. Utilisateur: /applyjob description="..."
         │
         ▼
2. Bot: Defer response (éviter timeout 3s)
         │
         ▼
3. Bot: POST /synthesize → Claude Server
         │
         ▼
4. Claude Server: claude -p "Analyse cette offre..."
         │
         ▼
5. Bot: Affiche embed Synthèse
         │
         ▼
6. Bot: Récupère CV actif de la DB
         │
         ▼
7. Bot: POST /match-skills → Claude Server
         │
         ▼
8. Bot: Affiche embed Compétences
         │
         ▼
9. Bot: POST /salary-analysis → Claude Server
         │
         ▼
10. Bot: Affiche embed Salaire
         │
         ▼
11. Bot: POST /generate-cv → Claude Server
         │
         ▼
12. Bot: Affiche embed CV Généré
```

### Workflow /sendcv

```
1. Utilisateur: /sendcv + fichier attaché
         │
         ▼
2. Bot: Valide le type de fichier (PDF/DOC/TXT)
         │
         ▼
3. Bot: Télécharge le fichier depuis Discord CDN
         │
         ▼
4. Bot: Sauvegarde dans data/cvs/
         │
         ▼
5. Bot: Upsert user dans SQLite
         │
         ▼
6. Bot: Insert CV dans base_cvs
         │
         ▼
7. Bot: POST /prompt (extraction texte PDF) → Claude
         │
         ▼
8. Bot: Update extracted_text dans base_cvs
         │
         ▼
9. Bot: Répond avec confirmation
```

## Sécurité

### Validation des entrées
- Types de fichiers autorisés: PDF, DOC, DOCX, TXT
- Vérification du content-type et de l'extension

### Permissions Discord
- Commandes admin: vérification côté serveur via `has_admin_permission()`
- Utilisation de `default_member_permissions(Permissions::ADMINISTRATOR)`

### Isolation
- Chaque utilisateur n'accède qu'à ses propres CVs
- Les requêtes DB filtrent par `user_id`

## Gestion des erreurs

### Niveaux d'erreur

```rust
pub enum CommandError {
    ResponseFailed(String),   // Erreur Discord API
    MissingParameter(String), // Paramètre manquant
    PermissionDenied,         // Accès refusé
    Internal(String),         // Erreur interne
}
```

### Stratégie de récupération
- Erreur Claude: Afficher un message utilisateur + log serveur
- Erreur DB: Propager avec message explicite
- Timeout: Message d'erreur gracieux

## Performance

### Optimisations
- Index SQLite sur les colonnes fréquemment requêtées
- Connection pool via `Arc<Mutex<Connection>>`
- Defer des réponses Discord pour les opérations longues

### Points de contention
- Mutex sur la connexion SQLite (un seul writer)
- Appels synchrones à Claude (120s timeout)

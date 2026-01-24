# Automate-CV-DiscordBot

Bot Discord pour automatiser la personnalisation de CV en fonction des offres d'emploi, propulsé par Claude AI.

## Contexte

En faisant ma recherche d'emploi, j'ai remarqué que j'ai les compétences pour postuler à plusieurs offres. Cependant, le pool de compétences est vaste et il est difficile de faire correspondre un seul CV à toutes les offres. J'ai donc décidé de créer un bot Discord qui automatise la personnalisation de mon CV en fonction des offres d'emploi.

## Fonctionnalités

- **Analyse de CV** - Upload et extraction automatique du contenu
- **Synthèse d'offres** - Extraction des informations clés (poste, entreprise, compétences)
- **Matching intelligent** - Score de compatibilité CV/offre avec détail des compétences
- **Analyse salariale** - Fourchettes marché et conseils de négociation
- **CV personnalisé** - Génération d'un CV adapté à chaque offre
- **Suivi des candidatures** - Tracking du statut de chaque candidature

## Commandes principales

| Commande | Description |
|----------|-------------|
| `/sendcv` | Uploader votre CV (PDF, DOC, TXT) |
| `/applyjob` | Analyser une offre et générer un dossier complet |
| `/status` | Voir vos candidatures |
| `/mystats` | Consulter vos statistiques |
| `/help` | Afficher toutes les commandes |

> Voir [doc/COMMANDS.md](doc/COMMANDS.md) pour la documentation complète des 15 commandes.

## Technologies

| Composant | Technologie |
|-----------|-------------|
| Bot Discord | Rust + Serenity 0.12 |
| Base de données | SQLite |
| IA | Claude Code (via HTTP wrapper Python) |
| Conteneurisation | Docker + Docker Compose |

## Quickstart

### Prérequis

- Docker & Docker Compose
- Token de bot Discord
- Image `claudeclode:latest` disponible

### Installation

```bash
# 1. Cloner le projet
git clone <repository_url>
cd Automate-CV-discordBot

# 2. Configurer les variables d'environnement
cp .env.example .env
# Éditer .env avec votre DISCORD_BOT_TOKEN

# 3. Créer le réseau et volume Docker
docker network create labnet
docker volume create rust

# 4. Lancer
docker compose up -d

# 5. Vérifier
docker compose ps
docker logs discord-cv-bot
```

> Voir [doc/DEPLOYMENT.md](doc/DEPLOYMENT.md) pour le guide complet de déploiement.

## Exemple d'utilisation

1. **Uploadez votre CV**
   ```
   /sendcv cv:mon_cv.pdf
   ```

2. **Analysez une offre d'emploi**
   ```
   /applyjob description:"Développeur Full Stack Senior chez TechCorp. CDI Paris. 5 ans exp Python, React..."
   ```

3. **Recevez 4 analyses**
   - Synthèse de l'offre
   - Compatibilité CV/Offre (score + détails)
   - Analyse salariale avec conseils
   - CV personnalisé pour cette offre

4. **Suivez vos candidatures**
   ```
   /status
   /updatestatus application_id:42 status:interview
   ```

## Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│                           DISCORD (Users)                             │
└───────────────────────────────┬──────────────────────────────────────┘
                                │ Slash Commands + Button Interactions
                                ▼
┌──────────────────────────────────────────────────────────────────────┐
│                      BOT DISCORD (Rust/Serenity)                      │
│  ┌────────────┐  ┌─────────────┐  ┌──────────┐  ┌─────────────────┐  │
│  │ commands/  │  │ services/   │  │   db/    │  │     main.rs     │  │
│  │  cv.rs     │  │ claude_     │  │  SQLite  │  │  EventHandler   │  │
│  │  jobs.rs   │  │ client.rs   │  │  wrapper │  │  TypeMap DI     │  │
│  │  admin.rs  │  │             │  │          │  │                 │  │
│  └────────────┘  └──────┬──────┘  └────┬─────┘  └─────────────────┘  │
└─────────────────────────┼──────────────┼─────────────────────────────┘
                          │ HTTP         │ File I/O
                          ▼              ▼
┌─────────────────────────────┐    ┌────────────────────────────────────┐
│   CLAUDE SERVER (Python)    │    │          DATA STORAGE               │
│   Port 8080                 │    │  ┌────────────┐  ┌──────────────┐  │
│   ─────────────────────     │    │  │ dbLookout/ │  │  data/cvs/   │  │
│   POST /synthesize          │    │  │  bot.db    │  │  fichiers CV │  │
│   POST /match-skills        │    │  └────────────┘  └──────────────┘  │
│   POST /salary-analysis     │    └────────────────────────────────────┘
│   POST /generate-cv         │
│   POST /extract-pdf         │
│   POST /generate-pdf        │
└──────────────┬──────────────┘
               │ subprocess
               ▼
┌─────────────────────────────┐
│      CLAUDE CODE CLI        │
│    claude -p "prompt"       │
└─────────────────────────────┘
```

> Voir [doc/DIAGRAMS.md](doc/DIAGRAMS.md) pour les diagrammes SysML détaillés.

> Voir [doc/ARCHITECTURE.md](doc/ARCHITECTURE.md) pour les détails techniques.

## Documentation

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](doc/ARCHITECTURE.md) | Architecture technique, composants, flux de données |
| [DIAGRAMS.md](doc/DIAGRAMS.md) | Diagrammes Mermaid (architecture, séquences, états, classes, ERD, mindmap) |
| [DEPLOYMENT.md](doc/DEPLOYMENT.md) | Guide de déploiement Docker et local |
| [TESTING.md](doc/TESTING.md) | Protocole de test complet |
| [API.md](doc/API.md) | Documentation API du serveur Claude (8 endpoints) |
| [COMMANDS.md](doc/COMMANDS.md) | Référence des commandes Discord (15 commandes) |

## Structure du projet

```
Automate-CV-discordBot/
├── src/
│   ├── main.rs           # Point d'entrée
│   ├── commands/         # Commandes Discord (15 commandes)
│   ├── db/               # Couche base de données SQLite
│   └── services/         # Client HTTP pour Claude
├── claude-server/        # Wrapper HTTP Python pour Claude CLI
├── doc/                  # Documentation technique
├── dbLookout/            # Base de données (généré)
├── data/cvs/             # CVs uploadés (généré)
├── Dockerfile            # Image du bot Rust
├── docker-compose.yml    # Orchestration
└── .env.example          # Template de configuration
```

## Développement

```bash
# Build
cargo build

# Tests
cargo test

# Run local (nécessite le serveur Claude)
export DISCORD_BOT_TOKEN="..."
export CLAUDE_API_URL="http://localhost:8080"
cargo run
```

> Voir [doc/TESTING.md](doc/TESTING.md) pour le protocole de test complet.

## Roadmap

- [x] Upload et extraction de CV (PDF/DOC/DOCX/TXT)
- [x] Synthèse d'offres d'emploi
- [x] Matching compétences CV/Offre
- [x] Analyse salariale avec conseils de négociation
- [x] Génération CV adapté (format structuré)
- [x] Compilation PDF du CV généré (reportlab + LaTeX fallback)
- [x] Suivi des candidatures avec boutons interactifs
- [x] Historique des changements de statut
- [x] Statistiques utilisateur
- [ ] Génération de lettres de motivation
- [ ] Rappels automatiques pour les relances
- [ ] Export des statistiques (CSV/JSON)
- [ ] Support multi-langue (EN/FR)

## Licence

Projet personnel - Usage privé

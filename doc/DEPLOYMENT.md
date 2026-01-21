# Guide de Déploiement

Ce guide détaille les étapes pour déployer le bot Automate-CV-DiscordBot en environnement de développement et de production.

## Prérequis

### Logiciels requis

| Logiciel | Version minimale | Vérification |
|----------|------------------|--------------|
| Docker | 20.10+ | `docker --version` |
| Docker Compose | 2.0+ | `docker compose version` |
| Git | 2.0+ | `git --version` |

### Ressources Discord

1. **Application Discord** sur [Discord Developer Portal](https://discord.com/developers/applications)
2. **Bot Token** (section Bot)
3. **Guild ID** du serveur de test (clic droit sur le serveur → Copier l'identifiant)

### Infrastructure Docker

```bash
# Créer le réseau externe
docker network create labnet

# Créer le volume externe pour les données
docker volume create rust
```

### Image de base Claude

L'image `claudeclode:latest` doit être disponible localement. Cette image contient le CLI Claude Code.

```bash
# Vérifier la présence de l'image
docker images | grep claudeclode
```

---

## Phase 1 : Configuration

### 1.1 Cloner le projet

```bash
cd /workspace/rust
git clone <repository_url> Automate-CV-discordBot
cd Automate-CV-discordBot
```

### 1.2 Configurer les variables d'environnement

```bash
# Copier le template
cp .env.example .env

# Éditer le fichier
nano .env
```

**Contenu du fichier `.env` :**

```bash
# OBLIGATOIRE - Token du bot Discord
DISCORD_BOT_TOKEN=votre_token_discord_ici

# OPTIONNEL - ID du serveur pour le mode développement
# Si défini, les commandes sont enregistrées uniquement sur ce serveur (instantané)
# Si non défini, les commandes sont enregistrées globalement (peut prendre 1h)
GUILD_ID=123456789012345678

# OPTIONNEL - URL du serveur Claude (défaut: http://claudecode:8080)
CLAUDE_API_URL=http://claudecode:8080

# OPTIONNEL - Niveau de log (error, warn, info, debug, trace)
RUST_LOG=info
```

### 1.3 Créer le bot Discord

1. Aller sur https://discord.com/developers/applications
2. Cliquer sur **New Application**
3. Nommer l'application (ex: "CV Automation Bot")
4. Section **Bot** :
   - Cliquer sur **Add Bot**
   - Copier le **Token** → coller dans `.env`
   - Désactiver "Public Bot" si usage privé
5. Section **OAuth2 → URL Generator** :
   - Scopes: `bot`, `applications.commands`
   - Bot Permissions:
     - Send Messages
     - Embed Links
     - Attach Files
     - Read Message History
     - Use Slash Commands
6. Copier l'URL générée et inviter le bot sur votre serveur

---

## Phase 2 : Déploiement Docker

### 2.1 Build des images

```bash
cd /workspace/rust/Automate-CV-discordBot

# Build complet (recommandé pour la première fois)
docker compose build --no-cache

# Build incrémental (plus rapide)
docker compose build
```

**Durée estimée :** 5-10 minutes (compilation Rust)

### 2.2 Lancement des services

```bash
# Mode interactif (voir les logs en direct)
docker compose up

# Mode détaché (en arrière-plan)
docker compose up -d
```

### 2.3 Vérification du déploiement

```bash
# Vérifier que les conteneurs tournent
docker compose ps

# Résultat attendu:
# NAME              STATUS          PORTS
# discord-cv-bot    Up X minutes
# claudecode        Up X minutes    0.0.0.0:8080->8080/tcp

# Vérifier les logs du bot
docker logs discord-cv-bot

# Résultat attendu:
# 🗄️  Connected to database: dbLookout/bot.db
# ✅ Database initialized successfully
# 🤖 Connected to Claude HTTP server
# 🚀 Starting bot...
# ✅ BotName is now online!
# 🔧 Registered X guild commands

# Health check du serveur Claude
curl http://localhost:8080/health
# {"status": "ok", "service": "claude-server"}
```

### 2.4 Commandes de gestion

```bash
# Arrêter les services
docker compose down

# Redémarrer un service spécifique
docker compose restart discord-bot
docker compose restart claudecode

# Voir les logs en temps réel
docker compose logs -f

# Logs d'un service spécifique
docker compose logs -f discord-bot
docker compose logs -f claudecode

# Reconstruire et relancer
docker compose up -d --build
```

---

## Phase 3 : Déploiement local (développement)

### 3.1 Installer Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup default stable
```

### 3.2 Build du projet

```bash
cd /workspace/rust/Automate-CV-discordBot

# Build debug (rapide, non optimisé)
cargo build

# Build release (lent, optimisé)
cargo build --release
```

### 3.3 Lancer le serveur Claude localement

```bash
# Terminal 1 - Serveur Claude
cd claude-server
python3 server.py

# Attendu:
# 🚀 Claude HTTP Server running on port 8080
```

### 3.4 Lancer le bot

```bash
# Terminal 2 - Bot Discord
cd /workspace/rust/Automate-CV-discordBot

# Définir les variables
export DISCORD_BOT_TOKEN="votre_token"
export GUILD_ID="votre_guild_id"
export CLAUDE_API_URL="http://localhost:8080"
export RUST_LOG=debug

# Lancer
cargo run

# Ou en mode release
cargo run --release
```

---

## Phase 4 : Production

### 4.1 Recommandations de sécurité

1. **Ne jamais exposer le port 8080** du serveur Claude sur Internet
2. **Utiliser des secrets Docker** au lieu de `.env` en production
3. **Activer les logs structurés** pour le monitoring
4. **Configurer des limites de ressources** dans docker-compose

### 4.2 Configuration production

```yaml
# docker-compose.prod.yml
services:
  discord-bot:
    deploy:
      resources:
        limits:
          cpus: '1'
          memory: 512M
    restart: always
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"

  claudecode:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
    restart: always
```

### 4.3 Lancement production

```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d
```

### 4.4 Sauvegarde des données

```bash
# Sauvegarder la base de données
docker cp discord-cv-bot:/app/dbLookout/bot.db ./backup/bot.db.$(date +%Y%m%d)

# Sauvegarder les CVs
docker cp discord-cv-bot:/app/data/cvs ./backup/cvs.$(date +%Y%m%d)
```

---

## Dépannage

### Le bot ne démarre pas

```bash
# Vérifier les logs
docker logs discord-cv-bot

# Erreurs courantes:
# - "Expected DISCORD_BOT_TOKEN" → Token manquant dans .env
# - "Failed to create client" → Token invalide
# - "Claude connection failed" → Serveur Claude non accessible
```

### Les commandes n'apparaissent pas

1. Attendre 1 heure si mode global (sans GUILD_ID)
2. Vérifier que le bot a les permissions `applications.commands`
3. Réinviter le bot avec les bons scopes

### Erreur "Claude timeout"

```bash
# Vérifier que le serveur Claude répond
curl http://localhost:8080/health

# Vérifier les ressources
docker stats claudecode
```

### Base de données corrompue

```bash
# Supprimer et recréer
docker compose down
rm -rf dbLookout/bot.db
docker compose up -d
```

---

## Mise à jour

### Mise à jour du code

```bash
# Arrêter
docker compose down

# Mettre à jour
git pull origin main

# Reconstruire
docker compose build --no-cache

# Relancer
docker compose up -d
```

### Mise à jour des dépendances Rust

```bash
# Mettre à jour Cargo.lock
cargo update

# Reconstruire
docker compose build --no-cache
```

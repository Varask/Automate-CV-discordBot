# Référence des Commandes Discord

Ce document détaille toutes les commandes slash disponibles dans le bot Automate-CV-DiscordBot.

---

## Vue d'ensemble

| Catégorie | Commande | Description |
|-----------|----------|-------------|
| **CV** | `/sendcv` | Uploader un CV |
| | `/deletecv` | Supprimer son CV actif |
| | `/listmycvs` | Lister ses CVs |
| **Candidature** | `/applyjob` | Analyser une offre et générer un CV adapté |
| | `/status` | Voir ses candidatures |
| | `/updatestatus` | Mettre à jour le statut d'une candidature |
| | `/history` | Historique des changements de statut |
| | `/mystats` | Voir ses statistiques |
| **Rappels** | `/setreminder` | Définir un rappel pour une candidature |
| | `/listreminders` | Lister ses rappels |
| | `/clearreminder` | Supprimer le rappel d'une candidature |
| | `/createreminder` | Créer un rappel libre |
| | `/deletereminder` | Supprimer un rappel |
| **IA (Legacy)** | `/synthesizeoffer` | Synthétiser une offre |
| | `/generateresume` | Générer un CV adapté |
| | `/generatecoverletter` | Générer une lettre de motivation |
| | `/generatemarketanalysis` | Analyse de marché |
| **Admin** | `/listcvs` | Lister tous les CVs |
| | `/getcv` | Récupérer le CV d'un utilisateur |
| | `/clearallcvs` | Supprimer tous les CVs (avec confirmation) |
| **Aide** | `/help` | Afficher l'aide |

---

## Commandes CV

### /sendcv

Uploade un CV pour qu'il soit utilisé lors des analyses de candidature.

**Usage:**
```
/sendcv cv:<fichier>
```

**Paramètres:**

| Nom | Type | Requis | Description |
|-----|------|--------|-------------|
| `cv` | Attachment | Oui | Fichier CV à uploader |

**Formats acceptés:**
- PDF (`.pdf`) - Recommandé
- Word (`.doc`, `.docx`)
- Texte (`.txt`)

**Taille maximale:** 25 Mo (limite Discord)

**Comportement:**
1. Valide le type de fichier
2. Télécharge le fichier depuis Discord
3. Sauvegarde localement dans `data/cvs/`
4. Désactive les anciens CVs de l'utilisateur
5. Extrait le texte via Claude (pour les PDFs)
6. Enregistre les métadonnées en base

**Exemples de réponses:**

✅ Succès:
```
✅ CV enregistré avec succès!

👤 Utilisateur: @VotreNom
📄 Fichier: mon_cv.pdf
📦 Taille: 45678 bytes
🆔 ID: 42
📝 ✅ Texte extrait avec succès

Utilisez /applyjob pour postuler à une offre avec ce CV.
```

❌ Type non supporté:
```
❌ Type de fichier non supporté: application/x-executable

Formats acceptés: PDF, DOC, DOCX, TXT
```

---

### /deletecv

Supprime le CV actif de l'utilisateur.

**Usage:**
```
/deletecv
```

**Paramètres:** Aucun

**Comportement:**
1. Récupère le CV actif de l'utilisateur
2. Supprime l'entrée en base de données
3. Supprime le fichier physique (échec non-bloquant, loggé)

**Exemples de réponses:**

✅ Succès:
```
🗑️ CV supprimé!

📄 Fichier: mon_cv.pdf
```

❌ Aucun CV:
```
❌ Aucun CV actif trouvé.

Utilisez /sendcv pour envoyer un CV.
```

---

### /listmycvs

Liste tous les CVs de l'utilisateur (actifs et inactifs).

**Usage:**
```
/listmycvs
```

**Paramètres:** Aucun

**Exemple de réponse:**

```
📋 Vos CVs (3 total)

✅ Actif mon_cv_v3.pdf
  └ ID: 42 | 45 Ko | 2025-01-21

⬜ Inactif mon_cv_v2.pdf
  └ ID: 38 | 42 Ko | 2025-01-15

⬜ Inactif mon_cv_v1.pdf
  └ ID: 25 | 38 Ko | 2025-01-10
```

---

## Commandes Candidature

### /applyjob

Commande principale du bot. Analyse une offre d'emploi et génère:
1. Une synthèse de l'offre
2. Une analyse de compatibilité avec votre CV
3. Une analyse salariale
4. Un CV personnalisé

**Usage:**
```
/applyjob description:<texte> [url:<url>] [company:<nom>] [title:<titre>] [notes:<texte>]
```

**Paramètres:**

| Nom | Type | Requis | Description |
|-----|------|--------|-------------|
| `description` | String | Oui | Description complète de l'offre |
| `url` | String | Non | URL de l'offre (affiché en lien cliquable) |
| `company` | String | Non | Nom de l'entreprise (override la détection IA) |
| `title` | String | Non | Titre du poste (override la détection IA) |
| `notes` | String | Non | Notes d'expérience à intégrer dans le CV généré |

**Comportement:**
1. Defer la réponse (opération longue, timeout global 10 min)
2. Appelle `/synthesize` sur le serveur Claude
3. Crée un thread Discord pour les résultats détaillés
4. Affiche l'embed de synthèse (vert) dans le canal principal
5. Récupère le CV actif de l'utilisateur
6. Appelle `/match-skills` sur le serveur Claude
7. Affiche l'embed de compétences (jaune) dans le thread
8. Appelle `/salary-analysis` sur le serveur Claude
9. Affiche l'embed salarial (orange) dans le thread
10. Appelle `/generate-cv` sur le serveur Claude
11. Envoie le PDF CV généré dans le thread
12. Met à jour l'embed de suivi avec boutons de statut

**Durée:** 30s à 5min selon la complexité

**Résultat:** Embed de suivi dans le canal + thread dédié avec tous les détails

#### Embed 1: Synthèse de l'offre (Vert)

```
📋 SYNTHÈSE DE L'OFFRE

🏢 Entreprise: TechCorp
💼 Poste: Développeur Full Stack Senior
📍 Lieu: Paris
📝 Contrat: CDI
💰 Salaire: 55-70k€

🎯 Compétences clés:
• Python (5 ans)
• React
• PostgreSQL

📖 Résumé:
TechCorp recherche un développeur expérimenté...
```

#### Embed 2: Analyse de compatibilité (Jaune)

```
🎯 ANALYSE DE COMPATIBILITÉ

Score de matching:
████████░░ 78%

✅ Compétences matchées:
✅ Python: 7 ans → Requis: 5 ans
✅ Django: 4 ans → Requis: requis
⚠️ PostgreSQL: 2 ans → Requis: 3 ans

❌ Compétences manquantes:
❌ Kubernetes (nice-to-have)

⭐ Points forts à mettre en avant:
⭐ Expérience Python supérieure aux attentes
⭐ Solide background Django
```

#### Embed 3: Analyse salariale (Orange)

```
💰 ANALYSE SALARIALE

💵 Salaire annoncé: 55k€ - 70k€

📊 Marché (EUR):
📉 Bas: 50k€
📊 Médian: 60k€
📈 Haut: 75k€

📝 Analyse:
Le salaire proposé se situe dans la fourchette haute...

💡 Conseils de négociation:
💡 Valorisez votre expérience supérieure à 5 ans
💡 Mentionnez vos certifications techniques
```

#### Embed 4: CV personnalisé (Bleu)

```
📄 CV PERSONNALISÉ GÉNÉRÉ

📝 Résumé des adaptations:
CV adapté pour le poste de Développeur Senior...

✨ Modifications apportées:
• Mise en avant de l'expérience Python (7 ans)
• Section Django déplacée en première position
• Ajout de mots-clés correspondant à l'offre

📥 Téléchargement:
La génération PDF sera disponible prochainement.
```

---

### /status

Affiche les candidatures de l'utilisateur.

**Usage:**
```
/status [filter:<statut>] [limit:<nombre>]
```

**Paramètres:**

| Nom | Type | Requis | Valeurs | Défaut |
|-----|------|--------|---------|--------|
| `filter` | Choice | Non | all, generated, applied, interview, offer, rejected, accepted | all |
| `limit` | Integer | Non | 1-25 | 10 |

**Exemple de réponse:**
```
📊 Your Applications (filter: all, limit: 10)

🟢 #42 Développeur Senior @ TechCorp
   Match: 78% | Généré: 2025-01-21

🟡 #38 DevOps Engineer @ StartupXYZ
   Match: 65% | Postulé: 2025-01-18
```

---

### /updatestatus

Met à jour le statut d'une candidature.

**Usage:**
```
/updatestatus application_id:<id> status:<statut> [note:<texte>]
```

**Paramètres:**

| Nom | Type | Requis | Description |
|-----|------|--------|-------------|
| `application_id` | Integer | Oui | ID de la candidature (depuis /status) |
| `status` | Choice | Oui | Nouveau statut |
| `note` | String | Non | Note additionnelle |

**Statuts disponibles:**
- `applied` - Candidature envoyée
- `interview` - Entretien programmé
- `offer` - Offre reçue
- `rejected` - Refusée
- `accepted` - Acceptée

**Exemple de réponse:**
```
🟢 Status Updated

Application #42 → interview

📝 Note: Premier entretien RH le 25/01
```

---

### /history

Affiche l'historique des changements de statut d'une candidature.

**Usage:**
```
/history application_id:<id>
```

**Paramètres:**

| Nom | Type | Requis | Description |
|-----|------|--------|-------------|
| `application_id` | Integer | Oui | ID de la candidature (depuis `/status`) |

**Exemple de réponse:**
```
📋 Historique candidature #42

🔄 generated → applied
   📅 2025-01-22 | 📝 Candidature envoyée via le site

🔄 applied → interview
   📅 2025-01-25 | 📝 Premier entretien RH le 28/01
```

---

### /mystats

Affiche les statistiques de candidature de l'utilisateur.

**Usage:**
```
/mystats
```

**Paramètres:** Aucun

**Exemple de réponse:**
```
📈 Your Statistics @VotreNom

📊 Total candidatures: 15
   • Générées: 5
   • Postulées: 8
   • Entretiens: 2
   • Offres: 0

🎯 Score moyen de matching: 72%

🏆 Top entreprises:
   1. TechCorp (3)
   2. StartupXYZ (2)
   3. BigCo (2)
```

---

## Commandes Rappels

### /setreminder

Définit un rappel de suivi pour une candidature existante.

**Usage:**
```
/setreminder application_id:<id> [days:<n>] [date:<YYYY-MM-DD>] [time:<HH:MM>]
```

**Paramètres:**

| Nom | Type | Requis | Description |
|-----|------|--------|-------------|
| `application_id` | Integer | Oui | ID de la candidature |
| `days` | Integer | Non | Délai en jours (1-90, défaut: 7) |
| `date` | String | Non | Date exacte au format YYYY-MM-DD |
| `time` | String | Non | Heure au format HH:MM (défaut: 09:00) |

**Note:** `days` et `date` sont mutuellement exclusifs. Si les deux sont fournis, `date` prime.

---

### /listreminders

Liste tous les rappels de l'utilisateur (candidatures et rappels libres).

**Usage:**
```
/listreminders
```

---

### /clearreminder

Supprime le rappel associé à une candidature.

**Usage:**
```
/clearreminder application_id:<id>
```

---

### /createreminder

Crée un rappel libre (non lié à une candidature).

**Usage:**
```
/createreminder message:<texte> [days:<n>] [date:<YYYY-MM-DD>] [time:<HH:MM>]
```

**Paramètres:**

| Nom | Type | Requis | Description |
|-----|------|--------|-------------|
| `message` | String | Oui | Texte du rappel |
| `days` | Integer | Non | Délai en jours (défaut: 1) |
| `date` | String | Non | Date exacte au format YYYY-MM-DD |
| `time` | String | Non | Heure au format HH:MM (défaut: 09:00) |

Le rappel sera posté dans le canal où la commande a été tapée.

---

### /deletereminder

Supprime un rappel libre par son ID.

**Usage:**
```
/deletereminder reminder_id:<id>
```

---

## Commandes IA (Legacy)

Ces commandes offrent un accès direct aux fonctionnalités IA, mais `/applyjob` les combine toutes.

### /synthesizeoffer

Synthétise une offre d'emploi.

**Usage:**
```
/synthesizeoffer description:<texte>
```

**Résultat:** Embed de synthèse (identique au premier embed de /applyjob)

---

### /generateresume

Génère un CV adapté à une offre.

**Usage:**
```
/generateresume job_description:<texte>
```

**Prérequis:** CV uploadé via `/sendcv`

**Résultat:** Embed avec score de matching et CV adapté

---

### /generatecoverletter

Génère une lettre de motivation.

**Usage:**
```
/generatecoverletter job_description:<texte> [application_id:<id>]
```

**Paramètres:**

| Nom | Type | Requis | Description |
|-----|------|--------|-------------|
| `job_description` | String | Oui | Description du poste |
| `application_id` | Integer | Non | ID de candidature à lier (depuis `/status`) |

**Prérequis:** CV uploadé via `/sendcv` (optionnel mais recommandé)

**Comportement avec `application_id`:**
- Sauvegarde la lettre dans la candidature en base
- Poste automatiquement la lettre dans le thread Discord de la candidature

**Résultat:** Embed avec la lettre de motivation générée

---

### /generatemarketanalysis

Analyse le marché de l'emploi basée sur le profil de l'utilisateur.

**Usage:**
```
/generatemarketanalysis
```

**Prérequis:** CV uploadé via `/sendcv`

**Résultat:** Embed avec analyse de marché, compétences demandées, fourchettes salariales

---

## Commandes Admin

Ces commandes nécessitent la permission Administrateur sur le serveur Discord.

### /listcvs

Liste tous les CVs stockés dans le bot.

**Usage:**
```
/listcvs
```

**Permission:** Administrateur

**Exemple de réponse:**
```
📋 All stored CVs:

@User1 - cv_123.pdf (45 Ko)
@User2 - resume.pdf (38 Ko)
@User3 - mon_cv.pdf (52 Ko)
```

---

### /getcv

Récupère le CV d'un utilisateur spécifique.

**Usage:**
```
/getcv user:<@utilisateur>
```

**Paramètres:**

| Nom | Type | Requis | Description |
|-----|------|--------|-------------|
| `user` | User | Oui | Utilisateur cible |

**Permission:** Administrateur

---

### /clearallcvs

Supprime tous les CVs stockés. **Action irréversible.**

**Usage:**
```
/clearallcvs
```

**Permission:** Administrateur

**Comportement:**
1. Affiche un message avec deux boutons : **Confirmer suppression** (rouge) et **Annuler** (gris)
2. Sur confirmation : supprime tous les CVs en base et les fichiers physiques, log l'action admin
3. Sur annulation : ferme le dialogue sans rien supprimer

---

## Commande Aide

### /help

Affiche la liste de toutes les commandes disponibles.

**Usage:**
```
/help
```

**Exemple de réponse:**
```
📚 Available Commands:

• /sendcv — Upload your CV to the bot
• /deletecv — Delete your CV from the bot
• /listmycvs — List your stored CVs
• /applyjob — Apply to a job: generates synthesis, tailored CV, and salary analysis
• /status — View your job application statuses
• /updatestatus — Update the status of a job application
• /mystats — View your application statistics
• /synthesizeoffer — Synthesize key information from a job description
• /generateresume — Generate a tailored resume based on job description and your CV
• /generatecoverletter — Generate a cover letter based on job description and your stored CV
• /generatemarketanalysis — Generate a market analysis based on job trends and your skills
• /listcvs — List all stored CVs (admin only)
• /getcv — Retrieve a specific CV by user (admin only)
• /clearallcvs — Delete all stored CVs (admin only)
• /help — Display help information about the bot's commands
```

---

## Gestion des erreurs

### Messages d'erreur communs

| Erreur | Cause | Solution |
|--------|-------|----------|
| "Missing parameter: X" | Paramètre requis absent | Ajouter le paramètre |
| "Database not found" | Erreur interne DB | Redémarrer le bot |
| "Claude client not found" | Serveur Claude inaccessible | Vérifier le conteneur claudecode |
| "Permission denied" | Droits insuffisants | Demander les droits admin |
| "CV not found" | Pas de CV uploadé | Utiliser /sendcv |

### Timeouts

Les commandes qui appellent Claude (toutes sauf /help, /listmycvs) peuvent prendre jusqu'à 2 minutes. Le bot "defer" automatiquement la réponse pour éviter le timeout Discord de 3 secondes.

---

## Bonnes pratiques

1. **Uploadez un CV complet** - Plus le CV est détaillé, meilleure sera l'analyse
2. **Utilisez le CV LinkedIn** - Il est généralement à jour et complet
3. **Collez l'offre complète** - Incluez toutes les sections de l'offre
4. **Trackez vos candidatures** - Utilisez `/updatestatus` pour suivre l'avancement
5. **Consultez vos stats** - `/mystats` vous aide à identifier les tendances

# 🤖 Automate-CV-DiscordBot

## 📋 Context

En faisant ma recherche d'emploi, j'ai remarqué que j'ai les compétences pour postuler à plusieurs offres. Cependant, le pool de compétences est vaste et il est difficile de faire correspondre un seul CV à toutes les offres. J'ai donc décidé de créer un bot Discord qui automatise la personnalisation de mon CV en fonction des offres d'emploi.

## ✨ Fonctionnalités

- 📄 Analyse du CV 
- 💼 Analyse de l'offre d'emploi
- 🎯 Personnalisation du CV en fonction de l'offre
- 📤 Envoi sur Discord
    - 📊 Synthèse de l'offre d'emploi
    - ✅ CV personnalisé avec les points clés mis en avant
    - 💰 Analyse salariale basée sur les données du marché avec suggestions de négociation

> 🔄 **À venir** : Suivi des candidatures avec rappels automatiques pour les relances

## 🛠️ Technologies Utilisées

- 🦀 **Rust**
- 🔗 **Discord API**
- 💾 **SQLite**
- 📚 Bibliothèques de traitement de texte et d'analyse de données
- 🧠 **Claude Code** (Dockerisé) pour analyser les offres et générer des CV personnalisés


# Exemple d'Utilisation 

1. Ajoutez le bot à votre serveur Discord.
2. Envoyez une offre d'emploi au bot via le canal dédié et la commande `/sendcv`.
    - Le bot enregistrera l'offre dans une base de données SQLite.
3. Envoyez l'offre d'emploi au bot via la commande `/applyjob` pour qu'il vous envoie:
    - Une synthèse de l'offre d'emploi.
    - Un CV personnalisé mettant en avant les compétences pertinentes depuis votre CV de base (conseil utilisez CV linkendIn il a tendance a etre a jour et plus complet).
    - Une analyse salariale avec des suggestions de négociation.
4. Verifiez le statut de vos candidatures avec la commande `/status`.

## Diagramme d'Architecture


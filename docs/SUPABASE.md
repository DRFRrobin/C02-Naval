# Configurer les comptes en ligne (Supabase + Google + Discord)

Le jeu reste sur GitHub Pages. Supabase fournit gratuitement : la connexion Google / Discord, la sauvegarde des profils (XP, statistiques), le matchmaking et le relais des parties. **Sans cette configuration, le jeu continue de fonctionner comme avant** (comptes locaux, connexion directe PeerJS).

Compter 20 à 30 minutes. Remplace `https://TON-PSEUDO.github.io/C02-Naval/` par l'adresse réelle de ton jeu.

## 1. Créer le projet Supabase
1. Compte gratuit sur <https://supabase.com>, puis **New project** (choisis une région proche, ex. Paris ou Francfort, et note le mot de passe de base de données : tu n'en auras pas besoin ensuite).
2. **SQL Editor → New query**, colle tout le contenu de [`supabase/schema.sql`](../supabase/schema.sql), clique **Run**. Le script peut être relancé sans danger.
3. **Project Settings → API** : note l'**URL du projet** et la clé **anon public**. Cette clé est publique par conception : la sécurité repose sur les règles du schéma, pas sur sa confidentialité. **Ne copie jamais la clé `service_role`.**

## 2. Autoriser l'adresse du jeu
**Authentication → URL Configuration** :
- **Site URL** : `https://TON-PSEUDO.github.io/C02-Naval/`
- **Redirect URLs** : ajoute la même adresse (et `http://localhost:8080/` pour tester en local).

## 3. Activer la connexion Google
1. <https://console.cloud.google.com> → crée un projet → **APIs & Services → OAuth consent screen** (type *External*, renseigne le nom de l'app et ton e-mail).
2. **Credentials → Create credentials → OAuth client ID**, type **Web application**.
   - **Authorized redirect URIs** : l'adresse affichée par Supabase dans **Authentication → Providers → Google** (`https://XXXX.supabase.co/auth/v1/callback`).
3. Copie le **Client ID** et le **Client secret** dans Supabase (**Authentication → Providers → Google**), active le fournisseur, enregistre.

## 4. Activer la connexion Discord
1. <https://discord.com/developers/applications> → **New Application**.
2. **OAuth2** : ajoute en *Redirects* la même adresse `https://XXXX.supabase.co/auth/v1/callback`. Copie le **Client ID** et le **Client Secret**.
3. Dans Supabase (**Authentication → Providers → Discord**), colle-les, active, enregistre.

## 5. Brancher le jeu
Dans `index.html`, renseigne les deux valeurs en haut du script :

```js
const CLOUD_CFG = Object.assign({ url: 'https://XXXX.supabase.co', key: 'eyJ…clé anon…' }, window.C02_CONFIG || {});
```

Commit, push : GitHub Pages se met à jour. L'écran de connexion propose alors Google et Discord.

## 6. (Facultatif) Garder le projet éveillé
Un projet gratuit se met en pause après environ 7 jours sans activité (les données sont conservées ; un clic le réveille). Pour l'éviter : dans le dépôt GitHub, **Settings → Secrets and variables → Actions**, ajoute `SUPABASE_URL` et `SUPABASE_ANON_KEY`. Le workflow `keepalive.yml` enverra une requête par semaine.

## Ce qui est stocké
Identifiant de connexion (géré par Supabase), pseudo, emoji, XP et statistiques. **Ni le nom réel, ni la photo, ni l'e-mail Google ou Discord ne sont recopiés dans la table des profils.** Chaque joueur peut supprimer son compte depuis Paramètres : tout est effacé.

## Limites de l'offre gratuite (à vérifier dans le tableau de bord)
200 connexions simultanées, 2 millions de messages Realtime par mois, 100 messages par seconde. Le jeu n'envoie qu'un message par action et un battement toutes les 10 s.

## Sécurité
- Les joueurs ne peuvent lire que leur propre profil, et ne peuvent rien écrire directement : tout passe par des fonctions qui valident les données et **calculent l'XP côté serveur** (plafonds par partie, une partie comptée toutes les 20 s au maximum).
- La partie reste jouée entre joueurs sans arbitre : un joueur déterminé peut mentir sur ses propres résultats, dans les limites de ces plafonds.

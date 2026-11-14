# ⚓ C02 Naval

Bataille navale dans le navigateur, sur PC ou téléphone. Sans installation, sans serveur de jeu.

**Jouer :** https://drfrrobin.github.io/C02-Naval/

## Modes de jeu

- **Adversaire aléatoire** : affronte un joueur en ligne au hasard. Si personne n'est trouvé en 45 s, un bot prend sa place.
- **Partie contre un bot** : entraînement immédiat contre une IA tactique (elle calcule les positions de bateaux les plus probables).
- **Hôte** : crée un salon privé et partage le code de 6 caractères (ou le lien d'invitation).
- **Rejoindre une partie** : saisis le code d'un ami.

Chaque bateau ennemi coulé donne une amélioration au choix : **Radar** (zone 2 × 2), **Frappe en croix** (5 cases) ou **Bouclier** (annule le premier coup sur un de tes bateaux).

## Règles personnalisables

Onglet **Règles** du tableau de bord :

- **Taille de la grille** : de 6 × 6 à 15 × 15.
- **Flotte** : nombre de bateaux de chaque taille (1 à 6 cases), 10 bateaux maximum et 40 % de la grille au plus.
- **Améliorations** : Radar, Frappe en croix et Bouclier activables séparément (toutes désactivées = bataille classique).
- **Chronomètre de tour** (désactivé, 15, 30, 60 ou 90 s) : à la fin du temps, un tir au hasard est joué. Il limite aussi le placement des bateaux à 90 s.
- **Intelligence artificielle** : autoriser ou non le bot (partie contre le bot et bot de secours du mode aléatoire), avec trois niveaux : facile, normal, difficile.

Ces règles s'appliquent aux parties « Hôte » et contre le bot ; l'invité qui rejoint un salon reçoit automatiquement les règles de l'hôte. L'adversaire aléatoire utilise toujours les règles standard. Avec des règles personnalisées, l'XP gagnée est divisée par deux.

## Comptes

Deux modes, selon que le service en ligne est configuré ou non (voir [`docs/SUPABASE.md`](docs/SUPABASE.md)) :

- **Avec Supabase (recommandé)** : connexion avec **Google** ou **Discord**, sans mot de passe. Le profil (pseudo, emoji), l'XP, les niveaux (de Mousse à Amiral) et les statistiques de carrière sont sauvegardés en ligne et suivent le joueur sur tous ses appareils. L'XP est calculée **côté serveur** avec des plafonds anti-abus. À la première connexion, on choisit son pseudo et son emoji, et on peut importer les statistiques d'un ancien compte local. Seuls l'identifiant de connexion, le pseudo, l'emoji et les statistiques sont stockés (ni nom réel, ni photo, ni e-mail). Un bouton permet de supprimer son compte.
- **Sans configuration** : comptes locaux dans le navigateur (pseudo + mot de passe PBKDF2-SHA-256), comme avant.
- Dans les deux cas, le **mode invité** reste disponible (sans XP). Contre le bot, l'XP gagnée est divisée par deux.

## Paramètres

Effets sonores (synthétisés, sans fichier audio), langue (français / anglais), **thème de couleurs** (Océan, Nuit, Forêt, Crépuscule, Clair), pseudo, mot de passe, avatar.

**Avatar** : uniquement un emoji choisi dans une liste fermée d'une cinquantaine d'emojis adaptés à partir de 12 ans (animaux, mer, espace, sport, smileys…). L'emoji reçu d'un adversaire est vérifié contre cette même liste.

**Responsive** : interface adaptée du petit téléphone (320 px) au grand écran, en portrait comme en paysage, avec prise en charge des zones sûres (encoche).

## Fiabilité du multijoueur

- **Relais en ligne (avec Supabase)** : les messages de la partie passent par un canal WebSocket (port 443) au lieu d'une connexion directe. Ça fonctionne sur pratiquement tous les réseaux (4G, Wi-Fi d'entreprise…), sans relais TURN. Le matchmaking « adversaire aléatoire » est une vraie file d'attente. Sans Supabase, le jeu garde la connexion directe PeerJS.
- **Reprise après coupure** : si le lien est perdu (changement de Wi-Fi, tunnel…), les deux joueurs voient « Reconnexion… » pendant 45 s au maximum. Les messages sont numérotés et renvoyés, la partie reprend là où elle s'était arrêtée, sans doublon ni perte. Si l'adversaire a quitté volontairement, c'est une victoire par forfait.
- **Relais TURN** (connexion directe uniquement) : sur les réseaux très fermés, ouvrir **Paramètres → Avancé** et saisir l'adresse d'un relais (`turn:serveur:3478` ou `turns:serveur:443?transport=tcp`), un identifiant et un mot de passe. Un compte gratuit chez un fournisseur (ExpressTURN, Metered…) suffit. Ces informations restent dans le navigateur. Pour l'activer pour tous les joueurs, renseigner la constante `MY_TURN` de `index.html`.

## Technique

- Un seul fichier de jeu : `index.html` (HTML, CSS et JavaScript). Le schéma de la base est dans `supabase/schema.sql`.
- Mode en ligne : [Supabase](https://supabase.com) (authentification, base de données, Realtime), bibliothèque `supabase-js` 2.45.4. Mode de secours sans configuration : connexion pair-à-pair (WebRTC) avec [PeerJS](https://peerjs.com) 1.5.4, dont le service public sert uniquement à mettre les navigateurs en relation.
- Sur les réseaux très fermés, la connexion directe peut échouer : un relais TURN personnel peut être ajouté dans la constante `MY_TURN` de `index.html`.
- Icônes Font Awesome 6, polices Inter et Orbitron (Google Fonts).

## Tests automatiques

```bash
npm install
npx playwright install chromium
npm test
```

Les tests (Playwright, `tests/game.test.js` et `tests/cloud.test.js`) vérifient la logique des règles, des parties complètes contre le bot (3 niveaux), le chronomètre, le multijoueur avec un PeerJS simulé (`tests/fake-peer.js`) : salon privé, adversaire aléatoire, reprise après coupure avec messages perdus, abandon, tentative de triche, et l'absence de débordement à 320 px dans tous les thèmes. Le mode en ligne est testé avec un faux Supabase (`tests/fake-backend.js`, `tests/fake-supabase.js`) : connexion, profil, XP, import, matchmaking, relais, coupure. Le schéma SQL est testé sur un vrai PostgreSQL (`supabase/tests/`). Ils tournent à chaque push et pull request via GitHub Actions (`.github/workflows/test.yml`).

## Tester en local

```bash
npm install
npm start
```

Puis ouvrir `http://localhost:8080`.

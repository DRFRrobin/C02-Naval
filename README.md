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
- **Intelligence artificielle** : autoriser ou non le bot (partie contre le bot et bot de secours du mode aléatoire), avec trois niveaux : facile, normal, difficile.

Ces règles s'appliquent aux parties « Hôte » et contre le bot ; l'invité qui rejoint un salon reçoit automatiquement les règles de l'hôte. L'adversaire aléatoire utilise toujours les règles standard. Avec des règles personnalisées, l'XP gagnée est divisée par deux.

## Comptes

- Pseudo + mot de passe (protégé par PBKDF2-SHA-256, jamais stocké en clair), avatar emoji.
- XP et niveaux (de Mousse à Amiral), statistiques de carrière : temps joué, victoires, défaites, taux, précision, séries…
- Les comptes sont enregistrés **dans le navigateur de l'appareil** : ils ne se synchronisent pas entre PC et téléphone, et effacer les données du site efface le compte. Le mot de passe protège le compte sur un appareil partagé, pas contre quelqu'un qui a accès au navigateur.
- Contre le bot, l'XP gagnée est divisée par deux.

## Paramètres

Effets sonores (synthétisés, sans fichier audio), langue (français / anglais), **thème de couleurs** (Océan, Nuit, Forêt, Crépuscule, Clair), pseudo, mot de passe, avatar.

**Avatar** : uniquement un emoji choisi dans une liste fermée d'une cinquantaine d'emojis adaptés à partir de 12 ans (animaux, mer, espace, sport, smileys…). L'emoji reçu d'un adversaire est vérifié contre cette même liste.

**Responsive** : interface adaptée du petit téléphone (320 px) au grand écran, en portrait comme en paysage, avec prise en charge des zones sûres (encoche).

## Technique

- Un seul fichier : `index.html` (HTML, CSS et JavaScript).
- Connexion pair-à-pair (WebRTC) avec [PeerJS](https://peerjs.com) 1.5.4. Le service public de PeerJS sert uniquement à mettre les navigateurs en relation.
- Sur les réseaux très fermés, la connexion directe peut échouer : un relais TURN personnel peut être ajouté dans la constante `MY_TURN` de `index.html`.
- Icônes Font Awesome 6, polices Inter et Orbitron (Google Fonts).

## Tester en local

```bash
npm install
npm start
```

Puis ouvrir `http://localhost:8080`.

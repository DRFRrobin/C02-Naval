# Cahier des charges — C02 Hub (serveur de jeux multi-jeux)

Version 0.2 — 2026-11-18 — statut : **à valider** (remplace la version 0.1)

## 0. Ce qui change depuis la version 0.1

| Sujet | v0.1 | v0.2 |
|---|---|---|
| Sauvegardes joueurs | non prévues | **Cloud Saves** complètes : slots, versions, compression, chiffrement optionnel, quotas par jeu (§6) |
| Stockage | une seule base SQLite | **Stockage hybride** : 2 bases SQLite séparées + coffre de fichiers adressés par contenu + palier froid optionnel (§7) |
| P2P en réseau fermé | STUN puis relais WebSocket | **Cascade de connectivité** avec « oracle de connectivité », super-pairs et IP masquée par défaut (§5.4) |
| Anti-triche | modules WebAssembly (wasmtime, lourd) | **Anti-triche à 4 paliers** : règles déclaratives, résultats co-signés, engagement-révélation, audits par échantillonnage. WASM léger (`wasmi`) uniquement en dernier recours (§8) |
| Synchronisation | non détaillée | **Flux de changements** à curseur, horloges hybrides, compteurs fusionnables (§5.6) |
| Manifeste | joueurs, modes, stats | + `[saves]`, `[anticheat]`, `[limits]`, `[relay]` (§9) |

## 1. Contexte et objectif

`server/` (Node.js) gère aujourd'hui les comptes de C02 Naval (voir `docs/SERVEUR.md`). Les parties passent en pair-à-pair via PeerJS. L'API tourne dans un conteneur sur un NAS TrueNAS, derrière un Cloudflare Tunnel.

**Objectif** : remplacer ce serveur par **un seul exécutable**, le C02 Hub. Il héberge plusieurs jeux en même temps (C02 Naval, [Zombie-Survivor-By-C02](https://github.com/Ghoster-001/Zombie-Survivor-By-C02), d'autres plus tard) et leur apporte un socle commun :

- un compte unique pour tous les jeux ;
- la mise en relation des joueurs (signalisation) ;
- la progression (XP, statistiques, succès) par jeu ;
- les **sauvegardes cloud** par jeu ;
- une base prête pour des amis et un chat (v2).

**Principes directeurs**

1. **Le serveur ne porte pas le jeu.** Il authentifie, met en relation, stocke et arbitre. Le trafic de jeu reste entre joueurs.
2. **Charge minimale.** Chaque octet et chaque cycle économisé = plus de joueurs sur la même machine.
3. **Fiable.** Aucune perte de données, redémarrage transparent, comportement prévisible sous charge.
4. **Aucun port ouvert sur la box.** Tout passe par un tunnel Cloudflare sortant. L'IP de la box n'est jamais publiée.
5. **Un seul fichier à déployer.** Pas de dépendance système.
6. **Zéro confiance dans le client**, mais **zéro coût** quand le client est honnête : la vérification coûteuse n'est déclenchée que sur soupçon.
7. **Extensible par manifeste.** Ajouter un jeu ne modifie pas le cœur.

## 2. Décisions retenues

| Sujet | Décision |
|---|---|
| Topologie multijoueur | **P2P + cascade de repli** (§5.4), relais désactivable par jeu |
| Langage | **Rust**, binaire unique, `tokio` + `axum` |
| Comptes | Pseudo + mot de passe, 2FA TOTP, clés de sécurité WebAuthn / passkeys |
| Données | SQLite (WAL), deux fichiers, plus un coffre de blobs (§7) |
| Hébergement | PC ou NAS à la maison + Cloudflare Tunnel |
| Exécution | Une instance seule ; plusieurs instances en v1.5 (§12) |

## 3. Périmètre

**Dans le périmètre**

- Exécutable serveur (Windows x64, Linux x64/arm64, image Docker pour le NAS).
- API HTTPS + WebSocket de signalisation.
- Système de modules de jeu déclaratifs (§9).
- Comptes, profils, sessions, 2FA.
- **Cloud Saves** (§6) et progression par jeu.
- Anti-triche proportionné (§8).
- Administration locale.
- Adaptation de C02 Naval et de Zombie Survivor (§14).

**Hors périmètre (v1)**

- Hébergement du trafic de jeu en temps réel dans le Hub.
- Chat, amis, canaux : v2, mais le modèle de données les anticipe.
- Paiements, boutique.

## 4. Architecture

```
Client (navigateur / .exe)
   │  HTTPS + WSS
   ▼
Cloudflare ──tunnel sortant──► cloudflared ──► C02 Hub (127.0.0.1:8787)
   │                                              │
   │                                              ├─ core.db   (comptes, sessions, profils)   SQLite WAL
   │                                              ├─ saves.db  (index des sauvegardes)        SQLite WAL
   │                                              ├─ vault/    (blobs de sauvegarde)          fichiers
   │                                              └─ [option] palier froid S3 / R2 chiffré
   │
   └── P2P direct (WebRTC) ◄── candidats ICE échangés par le Hub ──► autre joueur
        └─ sinon : cascade de repli (§5.4)
```

- **Runtime** : `tokio` multi-thread, `axum` pour HTTP et WebSocket. Écoute **locale** uniquement (`127.0.0.1`).
- **Modèle d'acteurs** : chaque salon est une tâche légère (`tokio::spawn`) avec sa boîte aux lettres. Aucun verrou global sur le chemin chaud. Les salons sont répartis par hachage sur N « shards ».
- **Horloges** : une roue de temporisateurs unique pour les expirations (reprise, files d'attente, sessions), au lieu d'un timer par objet.
- **`cloudflared`** : processus séparé, lancé et surveillé par le Hub (`tunnel.mode = "managed"`) ou déjà présent (`"external"`).
- **Mémoire** : état volatil réduit aux salons, files et présence. Reconstruit au redémarrage, rien de critique n'y vit.

### 4.1 Pourquoi le tunnel Cloudflare convient

- Il transporte HTTP(S) et WebSocket, ce qui suffit à de la signalisation et à un relais léger.
- Le trafic de jeu direct (WebRTC) ne passe jamais par Cloudflare.
- Ton IP reste cachée et le DHCP n'a aucun impact (tunnel sortant).
- **Limites à respecter** : pas de TCP/UDP brut sur le tunnel gratuit ; un volume important de média ou de transfert de gros fichiers est contraire aux conditions de Cloudflare. D'où : relais plafonné (§5.4) et sauvegardes de taille bornée avec transferts reprenables (§6).

## 5. Fonctionnalités du socle

### 5.1 Comptes et authentification

- Pseudo unique (3–20 caractères, normalisé en minuscules NFKC pour éviter les pseudos « sosies ») et mot de passe.
- **Argon2id** (paramètres réglés pour ≈ 100 ms sur la machine d'hébergement). Les anciens comptes scrypt sont migrés à la première connexion.
- **2FA TOTP** : QR code, 10 codes de secours à usage unique.
- **Passkeys WebAuthn** : plusieurs par compte, nommables, révocables.
- Politique par compte : mot de passe seul, mot de passe + 2FA, ou passkey seule.
- **Clés d'appareil** : à la première connexion, chaque appareil génère une paire **Ed25519** (stockée localement). La clé publique est rattachée au compte et sert à signer résultats de partie et sauvegardes (§6.3, §8).
- Sessions : jeton de 256 bits, seul son hachage est stocké. Liste et révocation par appareil. 60 jours glissants.
- Anti-force brute : verrouillage progressif par compte et limitation de débit par IP (`CF-Connecting-IP`, de confiance uniquement via le tunnel).
- Suppression de compte et export de toutes les données (RGPD), sauvegardes incluses.
- Récupération sans e-mail : codes de secours ou passkey. E-mail facultatif plus tard.
- Mode invité : jeton éphémère, aucune donnée conservée.

### 5.2 Profils et progression

- Profil global : pseudo, avatar, création.
- Données par jeu : XP, niveau, statistiques, succès, déclarés par le manifeste.
- **Le client n'envoie jamais l'XP.** Il envoie un résultat de partie ; le Hub valide (§8) puis calcule la récompense avec la formule du manifeste.

### 5.3 Mise en relation (signalisation)

- WebSocket authentifié sur `/ws`, un seul par client pour tous les jeux.
- **Salon privé** : code de 6 caractères, lien d'invitation.
- **Adversaire aléatoire** : file par jeu et par mode, avec repli sur un bot après un délai réglable.
- **Appariement conscient du réseau** : le Hub tient compte de la compatibilité de connexion (§5.4) et de la région, pour réduire la latence et le recours au relais.
- Présence, reprise après coupure (60 s) et détection d'abandon.
- **Jeton de rendez-vous** : les adresses ne sont échangées qu'entre partenaires, via des candidats ICE signés par le Hub et valables 2 minutes.

### 5.4 Connectivité P2P : la « cascade adaptative »

**Problème.** Un P2P pur échoue derrière des NAT stricts, des réseaux d'entreprise ou d'école, ou du CGNAT. Le tunnel Cloudflare ne transporte pas d'UDP. Et l'échange d'IP expose chaque joueur à ses adversaires.

**Invention : l'oracle de connectivité + la cascade.** Le Hub décide avant la partie, plutôt que d'attendre un échec ICE de 10 à 30 secondes.

1. **Sonde à la connexion (≈ 300 ms, en tâche de fond).** Le client interroge deux STUN (Cloudflare et un second au choix) et rapporte au Hub un résumé tenant en quelques octets : type de NAT estimé (ouvert, cône, symétrique), UDP disponible ou non, TCP 443 sortant disponible ou non, IPv6 disponible ou non.
2. **Matrice de compatibilité.** À l'appariement, le Hub calcule la meilleure voie pour la paire de joueurs à partir de ces résumés (table de décision, pas de calcul lourd).
3. **Cascade, de la plus économique à la plus robuste :**

| Palier | Voie | Coût serveur | Quand |
|---|---|---|---|
| A | P2P direct WebRTC (UDP, ICE + STUN) | nul | NAT compatibles, IPv6 ou NAT ouvert |
| B | P2P + **TCP hole punching** / WebRTC sur TCP | nul | UDP bloqué mais TCP sortant possible |
| C | **Super-pair** : un joueur de la même partie à NAT ouvert sert de relais à l'autre | nul pour le Hub | un des deux joueurs est joignable, l'autre non |
| D | **Relais du Hub** : trames WebSocket opaques dans le tunnel | faible, plafonné | tout le reste, jeux tour par tour surtout |
| E | TURN externe (optionnel, configuré) | externe | jeux temps réel avec réseaux fermés |

4. **Le relais du Hub (palier D) ne lit jamais les trames.** Elles sont chiffrées de bout en bout entre les deux joueurs (clé de session échangée pendant la signalisation, ECDH X25519 + ChaCha20-Poly1305). Le Hub copie des octets, sans parser. Résultat : faible CPU, pas de fuite de contenu.
5. **Budget de relais.** Plafond par jeu, par compte et global (octets par seconde, nombre de salons). Au-delà, nouvelle partie refusée proprement avec un message clair, jamais de dégradation des autres jeux.
6. **IP masquée par défaut** pour les parties aléatoires : le palier D (ou C) est utilisé de préférence tant que les joueurs ne sont pas amis ou ne partagent pas un salon privé. Le P2P direct (qui expose l'IP) est réservé aux amis et aux salons privés, ou activé volontairement par le joueur.
7. **Bascule en cours de partie** : si la voie directe se dégrade (perte de paquets, latence), le client peut demander un passage au palier suivant sans quitter la partie (migration de transport, sans réinitialiser l'état du jeu).
8. **Télémétrie locale** : taux de réussite de chaque palier par jeu, visible dans l'administration, pour ajuster les politiques.

Politique par jeu dans `[relay]` du manifeste : paliers autorisés, plafonds, IP masquée par défaut ou non.

### 5.5 Administration

- Console locale `http://127.0.0.1:PORT/admin`, accessible uniquement depuis la machine (hors tunnel).
- Comptes (blocage, réinitialisation 2FA), jeux actifs, salons, journaux, métriques, quotas de sauvegarde par joueur, intégrité des blobs, sauvegarde manuelle.
- Équivalent en ligne de commande (`c02hub admin ...`).

### 5.6 Synchronisation multi-appareils

**Problème.** Un joueur utilise plusieurs appareils (navigateur, .exe, mobile). Il faut que profil, progression et sauvegardes convergent sans conflits ni gros transferts.

**Mécanisme : flux de changements à curseur.**

- Chaque modification côté serveur reçoit un **numéro de séquence global** croissant (table `changes`).
- Le client mémorise un curseur et demande `GET /api/sync?since=N`. Il reçoit uniquement ce qui a changé (profil, métadonnées de sauvegardes, succès), jamais les blobs.
- Le WebSocket envoie une simple notification « il y a du nouveau » (quelques octets) : pas de polling.
- **Horloge hybride (HLC)** pour les champs fusionnables (préférences, pseudo d'affichage) : dernier écrivain gagnant, sans dépendre des horloges des appareils.
- **Compteurs fusionnables** pour les statistiques (parties jouées, victoires) : un compteur par appareil, la valeur affichée est la somme. Deux appareils hors ligne ne perdent donc aucune incrémentation.
- Les sauvegardes, elles, utilisent le contrôle de version optimiste de §6.3 car leur contenu est opaque.

## 6. Sauvegardes joueurs (Cloud Saves)

### 6.1 Modèle fonctionnel

- Chaque jeu déclare ses règles de sauvegarde dans `[saves]` (§9).
- Un joueur possède, **par jeu**, jusqu'à N **slots** nommés (`slot_id` : `auto`, `1`, `2`, ... ou nom libre de 1 à 32 caractères).
- Une sauvegarde est un **blob opaque** accompagné de **métadonnées** :
  - `schema` (version du format de jeu), `game_version`, `created_at`, `device_id` ;
  - `label` court et `summary` JSON borné (niveau, durée de jeu, capture miniature de ≤ 8 Ko facultative) pour l'affichage sans télécharger le blob ;
  - `size_raw`, `size_stored`, `sha256`, `compression`, `encrypted`.
- **Historique** : le Hub conserve les `keep_versions` dernières versions de chaque slot (retour en arrière possible) et une corbeille de 30 jours pour les suppressions.
- Actions : lister, lire, écrire, supprimer, restaurer une version, renommer, copier d'un slot à un autre.

### 6.2 API

| Méthode | Route | Rôle |
|---|---|---|
| GET | `/api/saves/{game}` | liste des slots avec métadonnées (sans blobs), quotas utilisés |
| GET | `/api/saves/{game}/{slot}` | dernière version (blob, avec `ETag`, `Range` pris en charge) |
| GET | `/api/saves/{game}/{slot}/versions` | historique |
| PUT | `/api/saves/{game}/{slot}` | écriture avec `If-Match: <version>` (contrôle optimiste) |
| POST | `/api/saves/{game}/{slot}/uploads` | session de transfert **reprenable** (gros blobs ou réseau instable) |
| POST | `/api/saves/{game}/{slot}/restore` | restaure une version antérieure comme nouvelle version |
| DELETE | `/api/saves/{game}/{slot}` | suppression (corbeille) |

Toutes les routes exigent la session et le jeton de jeu ; elles sont limitées en débit.

### 6.3 Contrôle de version optimiste (zéro perte)

- Chaque slot porte un **numéro de version** entier et un **hachage parent**. Une écriture déclare `If-Match: <version_connue>` et le `parent_sha256`.
- Si la version du serveur est plus récente → `409 Conflict`, avec la **description du conflit** : métadonnées des deux côtés (version serveur, version locale, appareil, date, résumé). **Le serveur ne perd jamais l'une des deux versions** : la version « perdante » est conservée dans l'historique.
- Résolution côté client, au choix du jeu (`conflict` dans le manifeste) :
  - `ask` : le joueur choisit (« garder celle du cloud », « garder celle de cet appareil », « garder les deux ») ;
  - `newest` : la plus récente gagne (jeux sans enjeu) ;
  - `highest_progress` : comparaison d'un champ du `summary` (ex. `level`, `playtime`) ;
  - `merge` : le jeu fournit sa fusion côté client à partir des deux blobs et du dernier ancêtre commun (le hachage parent permet de le retrouver).
- **Écriture atomique et idempotente** : chaque écriture porte un `request_id` ; un renvoi après coupure ne crée pas de doublon.
- **Signature d'appareil** : le client signe `(game, slot, version, sha256)` avec sa clé Ed25519. Le Hub vérifie. Cela prouve qui a écrit quoi et détecte une corruption en transit.

### 6.4 Compression et chiffrement

- **Compression `zstd`** côté client (niveau 3 par défaut, niveau et seuil réglables). Le Hub n'a pas à compresser : il économise le CPU.
  - Les blobs déjà compressés (détectés par un en-tête magique) ne sont pas recompressés.
- **Chiffrement de bout en bout facultatif** (`encryption = "optional"` ou `"required"` par jeu) : le client chiffre avec ChaCha20-Poly1305. La clé est dérivée d'un secret local (passkey, ou phrase de récupération).
  - Avantage : même si le serveur est compromis, les sauvegardes restent illisibles.
  - Conséquence : le Hub ne peut plus valider le contenu. Le jeu qui exige une validation serveur (anti-triche sur la sauvegarde) choisit `encryption = "off"`, ou ne valide que les métadonnées signées.
- Les blobs sont toujours **chiffrés au repos** côté serveur avec une clé maîtresse du Hub si le chiffrement de bout en bout n'est pas utilisé.

### 6.5 Quotas stricts et configurables

Les quotas sont déclarés **par jeu** dans `game.toml` (voir §9) avec des **plafonds globaux** dans `c02hub.toml`. Le plus restrictif s'applique.

| Quota | Rôle | Valeur par défaut |
|---|---|---|
| `max_slots` | nombre de slots par joueur et par jeu | 5 |
| `max_bytes_per_save` | taille maximale **stockée** d'un blob | 256 Ko |
| `max_total_bytes` | volume total par joueur et par jeu (versions incluses) | 8 Mo |
| `keep_versions` | versions conservées par slot | 5 |
| `max_writes_per_hour` | écritures par joueur, par jeu | 60 |
| `max_decompressed_ratio` | rapport maximal taille décompressée / compressée déclaré (anti « bombe ») | 50 |
| `global.max_total_bytes` | volume maximal de tout le coffre | 20 Go |
| `global.max_total_per_user` | volume maximal d'un joueur, tous jeux confondus | 64 Mo |

- Dépassement → `413` ou `429` avec un code d'erreur lisible (`quota_slots`, `quota_bytes`, `rate`) et les valeurs courantes pour que le client affiche un message utile.
- Le **volume total est tenu à jour de manière incrémentale** (compteur par joueur et par jeu dans `saves.db`), jamais recalculé par parcours.
- L'administration peut relever un quota pour un joueur précis (exception tracée).

## 7. Stratégie de stockage : le « stockage hybride »

**Problème.** Mettre des blobs dans SQLite en mode WAL fait grossir le fichier WAL, allonge les points de contrôle (`checkpoint`) et bloque les écritures des comptes pendant les gros transferts. À l'inverse, des fichiers seuls perdent l'atomicité transactionnelle.

**Solution retenue : séparation stricte des rôles.**

| Élément | Support | Pourquoi |
|---|---|---|
| Comptes, sessions, profils, 2FA, journaux d'audit | `core.db` (SQLite WAL) | petites lignes, forte concurrence, transactions |
| Index des sauvegardes (slots, versions, quotas, curseur de changements) | `saves.db` (SQLite WAL) | **fichier distinct** : une rafale de sauvegardes ne ralentit jamais la connexion des joueurs |
| **Blobs** de sauvegarde | `vault/` : fichiers **adressés par leur contenu** | le WAL reste petit, copies et sauvegardes simples |
| Petits blobs ≤ 4 Ko | colonne `inline` de `saves.db` | évite un fichier par miette de donnée |
| Archives anciennes (option) | palier **froid** S3 / Cloudflare R2, chiffré | libère le disque local, sans changer l'API |

### 7.1 Coffre de fichiers (`vault/`)

- Nom de fichier = `sha256` du blob stocké, rangé en `vault/ab/cd/abcd…` (deux niveaux de sous-dossiers).
- **Écriture atomique** :
  1. écrire dans `vault/tmp/<uuid>` ;
  2. `fsync` du fichier ;
  3. vérifier le `sha256` ;
  4. `rename` vers l'emplacement final (atomique sur le même système de fichiers) ;
  5. `fsync` du dossier ;
  6. **seulement ensuite**, transaction `saves.db` qui crée la nouvelle version et incrémente les compteurs.
- Un crash entre 4 et 6 laisse au pire un **blob orphelin**, jamais une version qui pointe vers un fichier absent. Les orphelins sont nettoyés par le ramasse-miettes.
- **Déduplication gratuite** : deux joueurs (ou deux versions) avec un contenu identique partagent le même fichier ; un compteur de références est tenu dans `saves.db`.
- **Ramasse-miettes** (tâche de fond, basse priorité) : supprime les blobs sans référence depuis plus de 24 h et nettoie `tmp/`. Les versions au-delà de `keep_versions` et la corbeille de plus de 30 jours sont purgées.
- **Contrôle d'intégrité** hebdomadaire : relit un échantillon de blobs, recompare le `sha256`, signale toute corruption en administration. Un blob corrompu est restauré depuis la sauvegarde ou le palier froid.
- Droits : le dossier `vault/` n'est lisible que par le compte du Hub.

### 7.2 Réglages SQLite

- `journal_mode = WAL`, `synchronous = NORMAL` sur `saves.db` ; `synchronous = FULL` sur `core.db` pour ne jamais perdre un compte validé.
- `wal_autocheckpoint` ajusté et point de contrôle forcé hors pointe.
- Écrivain unique par base (file de commandes), plusieurs lecteurs. Les écritures sont **regroupées** en une transaction par lot de quelques millisecondes.
- `busy_timeout` explicite, requêtes préparées en cache.

### 7.3 Sauvegarde et reprise après sinistre

- Quotidien : `VACUUM INTO` / API de sauvegarde SQLite pour `core.db` et `saves.db`, plus copie des **nouveaux** blobs uniquement (le coffre est adressé par contenu, donc incrémental par nature).
- Rotation 14 jours, vérification de restauration automatique chaque semaine dans un dossier temporaire.
- **Palier froid optionnel** : réplication chiffrée vers S3/R2 de la base et des blobs. Le Hub ne conserve alors en local que les blobs récents. Les accès restent transparents pour les clients.
- Restauration documentée par `c02hub restore` et testée dans les critères d'acceptation.

## 8. Anti-triche proportionné : « confiance échelonnée »

**Problème.** Valider chaque partie avec un module WebAssembly complet (wasmtime ajoute plusieurs dizaines de Mo au binaire et de la RAM) est disproportionné pour un serveur léger. La plupart des joueurs sont honnêtes.

**Invention : 4 paliers, du moins cher au plus cher.** Le Hub ne monte d'un palier que si le précédent ne suffit pas.

| Palier | Mécanisme | Coût | Rôle |
|---|---|---|---|
| **T0 : règles déclaratives** | Expressions du manifeste évaluées par un évaluateur minuscule (bornes, durées, cadences, formule d'XP) | ≈ 0 | filtre 95 % des résultats absurdes |
| **T1 : résultats co-signés** | Chaque joueur signe le résultat final (Ed25519, clé d'appareil). Le Hub n'accepte que si les deux signatures **concordent** | très faible | empêche l'auto-attribution d'une victoire ou d'un gain |
| **T2 : engagement-révélation** | Mécanisme propre aux jeux à information cachée (voir ci-dessous) | faible | détecte la triche en jeu d'information cachée |
| **T3 : audit par échantillonnage** | Rejeu du journal de coups par un vérificateur ; échantillon aléatoire + tout joueur suspect | variable, **à la demande** | vérité de dernier recours |

### 8.1 Détails

- **T0 – règles déclaratives** (`[anticheat.rules]`) : expressions simples comme `duration_s >= 20`, `shots <= 100`, `score <= 5000 * duration_s`. L'évaluateur est une bibliothèque Rust de quelques dizaines de Ko (type `evalexpr`), **sans code arbitraire**.
- **T1 – co-signature** : à la fin d'une partie, chaque pair envoie au Hub `{partie, joueurs, résultat, hash_du_journal}` signé. Un désaccord ouvre automatiquement un dossier : le résultat est suspendu, aucun XP n'est accordé, les deux journaux sont conservés.
- **T2 – engagement-révélation pour C02 Naval** : avant le premier tir, chaque joueur publie l'**empreinte** (hachage + sel secret) de la position de sa flotte. Chaque réponse « touché / à l'eau » est cohérente ou non avec cette empreinte. En fin de partie, chacun **révèle** sa flotte et son sel : l'adversaire (et le Hub) vérifie l'empreinte et rejoue tous les tirs. Un joueur qui a menti sur un tir est **prouvé tricheur de manière irréfutable**. Aucun module n'est requis : la vérification est un simple calcul de hachage.
- **T3 – audit** : le Hub dispose d'un trait Rust `Verifier` (rejeu déterministe du journal). Deux façons de l'obtenir :
  1. **Vérificateurs natifs** compilés dans le binaire, activables par `cargo feature` (ex. `naval`). Zéro surcoût hors utilisation.
  2. **Plugin WebAssembly léger** (`wasmi`, interpréteur d'environ 1 Mo, sans compilation à la volée) chargé **à la demande** puis déchargé. Pour les jeux tiers.
  - Pas de plugin → T3 est simplement indisponible pour ce jeu.
- **Zombie Survivor** (jeu en temps réel) : le Hub délivre au début de la partie une **graine signée**. Le jeu, déterministe à partir de cette graine, enregistre un journal d'entrées compact. Le résultat est accepté sous T0 et T1, avec **audit T3 par échantillonnage** sur un pourcentage réglable et sur tout score dépassant le 99e centile du jeu.
- **Réputation** : un score de confiance par compte et par jeu (durée du compte, parties vérifiées, litiges). Un score bas augmente la probabilité d'audit ; un score élevé la réduit, ce qui garde la vérification économe.
- **Sanctions** : résultat annulé, remise à zéro de la progression suspecte, blocage temporaire ou définitif. Toute sanction est tracée dans `audit_log` et réversible par l'administrateur.

## 9. Système multi-jeux : manifestes

Chaque jeu est décrit par un manifeste. Ajouter un jeu revient à ajouter un fichier et, éventuellement, un vérificateur.

```toml
# games/c02-naval/game.toml
id            = "c02-naval"
name          = "C02 Naval"
min_players   = 2
max_players   = 2
modes         = ["random", "private"]
bot_fallback_seconds = 45
allowed_origins = ["https://drfrrobin.github.io"]

[relay]
tiers          = ["direct", "tcp", "superpeer", "hub"]   # paliers autorisés (§5.4)
mask_ip_random = true
max_relay_rooms = 200
max_relay_bytes_per_s_room = 4096

[stats]
xp     = "counter"
wins   = "counter"
losses = "counter"

[progression]
xp_formula = "win ? 100 + sunk_ships * 10 : 30 + sunk_ships * 5"

[saves]
enabled              = true
max_slots            = 3
max_bytes_per_save   = 65536        # octets stockés (après compression)
max_total_bytes      = 2097152      # 2 Mo par joueur
keep_versions        = 5
max_writes_per_hour  = 30
max_decompressed_ratio = 20
encryption           = "optional"   # off | optional | required
conflict             = "ask"        # ask | newest | highest_progress | merge
summary_fields       = ["level", "xp"]

[anticheat]
tiers          = ["rules", "cosign", "commit_reveal"]
audit_sample_pct = 2
[anticheat.rules]
duration_s = ">= 20"
shots      = "<= 100"

[limits]
max_rooms        = 500
max_ws_msg_bytes = 2048
max_msgs_per_s   = 20
```

- **Niveau 1 : déclaratif.** Joueurs, modes, relais, statistiques, formules d'XP, sauvegardes, règles. Ce niveau suffit à la plupart des jeux.
- **Niveau 2 : vérificateur** natif (feature Cargo) ou plugin `wasmi` chargé à la demande (§8).
- **Isolation entre jeux** : espaces de noms distincts pour salons, files, statistiques, sauvegardes et quotas. Un jeu peut être mis hors ligne sans toucher aux autres.
- **Rechargement à chaud** des manifestes via l'administration, sans coupure des parties en cours. Une erreur de manifeste refuse le rechargement et conserve l'ancien.
- **Contrat de protocole** : tous les messages portent `{ v, game, type }`, avec négociation de version et message « mise à jour requise » pour un client trop ancien.
- **SDK client** (JavaScript, ≈ 5 Ko, plus un client Rust pour la version Windows) : `hub.login()`, `hub.createRoom()`, `hub.join(code)`, `hub.findMatch()`, `hub.saves.list()/read()/write()`, `hub.reportResult()`. Il gère la reconnexion, la sonde de connectivité, la compression et la signature.

## 10. Modèle de données

**`core.db`**

| Table | Contenu |
|---|---|
| `users` | id, pseudo, `pseudo_norm` (unique), Argon2id, statut, création |
| `user_totp`, `user_recovery_codes` | secret TOTP chiffré, codes de secours hachés |
| `user_webauthn` | clé publique, compteur, nom, dernière utilisation |
| `devices` | id, user, clé publique Ed25519, nom, dernière activité |
| `sessions` | hash du jeton, user, appareil, expiration |
| `game_profiles` | (user, game) → XP, niveau, succès, réputation |
| `stat_counters` | (user, game, clé, appareil) → valeur (compteurs fusionnables) |
| `match_results` | partie, jeu, participants, résultat signé, statut de vérification |
| `disputes` | parties litigieuses et journaux conservés |
| `audit_log` | connexions, échecs, sanctions, actions d'administration |
| *v2 :* `friends`, `channels`, `messages` | prévus, non créés en v1 |

**`saves.db`**

| Table | Contenu |
|---|---|
| `save_slots` | (user, game, slot) → version courante, étiquette, date |
| `save_versions` | slot, version, `parent_sha256`, `sha256`, tailles, compression, chiffrement, appareil, signature, `inline` (si ≤ 4 Ko), état (active, historique, corbeille) |
| `blob_refs` | `sha256` → compteur de références, taille, palier (local ou froid) |
| `quota_usage` | (user, game) → octets, nombre de slots, écritures de l'heure |
| `uploads` | sessions de transfert reprenables, expiration |
| `changes` | séquence globale pour la synchronisation (§5.6) |

- Migrations versionnées, appliquées au démarrage avec sauvegarde préalable. Les secrets TOTP et la clé du coffre sont chiffrés avec une clé maîtresse hors base (`C02_MASTER_KEY` ou fichier à droits restreints).

## 11. Exigences non fonctionnelles

### 11.1 Performance (cibles à mesurer)

| Indicateur | Cible |
|---|---|
| Mémoire au repos | ≤ 30 Mo |
| Mémoire avec 5 000 connexions WebSocket | ≤ 250 Mo |
| CPU au repos | ≈ 0 % |
| Taille du binaire | **≤ 15 Mo** (sans wasmtime ; plugin `wasmi` en option ≈ +1 Mo) |
| Latence de signalisation (Hub → pair), hors réseau | ≤ 20 ms au 99e centile |
| Écriture d'une sauvegarde de 64 Ko | ≤ 30 ms de traitement serveur au 99e centile |
| Capacité visée sur mini-PC ou NAS modeste | ≥ 5 000 joueurs connectés, ≥ 100 requêtes/s |

- Messages WebSocket compacts (binaire ou JSON court), diffusion ciblée seulement.
- Chemin chaud sans allocation par message, journalisation asynchrone.
- Banc d'essai de charge livré avec le projet (clients simulés : signalisation, relais, sauvegardes).

### 11.2 Fiabilité

- Redémarrage automatique (service Windows, systemd ou politique Docker).
- Arrêt propre sur signal : écritures terminées, clients prévenus, reconnexion automatique (délai exponentiel avec variation aléatoire).
- `/api/health` (vivacité) et `/api/ready` (bases et coffre joignables).
- Idempotence : un résultat de partie ou une sauvegarde n'est accepté qu'une fois par identifiant de requête.
- Limites partout : taille des corps, messages par seconde, salons par compte, écritures de sauvegarde par heure.
- **Pression sur le disque** : sous un seuil d'espace libre, le Hub passe en lecture seule pour les sauvegardes et alerte, plutôt que de corrompre.

### 11.3 Sécurité

- Le Hub n'écoute qu'en local ; en-têtes de sécurité (CSP, `X-Content-Type-Options`).
- CORS restreint aux origines des manifestes.
- Entrées validées strictement, aucune requête SQL concaténée, journaux sans secret.
- **Anti « bombes de décompression »** : le blob est stocké tel quel, le Hub ne décompresse jamais ; le rapport annoncé est borné et vérifié par l'en-tête zstd.
- **Chiffrement au repos** des blobs (clé maîtresse) et de bout en bout en option.
- Dépendances auditées (`cargo audit`, `cargo deny`) à chaque compilation ; binaires signés avec sommes SHA-256 publiées.
- IP des joueurs : jamais stockées durablement (mémoire, 24 h, pour la limitation de débit). Politique de confidentialité à rédiger.
- Cloudflare WAF : limitation de débit sur `/api/login`, `/api/register`, WebAuthn et écriture de sauvegardes.

### 11.4 Observabilité

- Journaux JSON structurés avec rotation.
- Métriques Prometheus sur `/metrics` (accès local) : connexions, salons, files, taux de réussite par palier de connectivité, relais en cours, latence des écritures, taille des bases et du coffre, litiges, audits.

### 11.5 Maintenabilité

- Cœur et modules de jeu séparés. Tests unitaires, d'intégration (clients simulés), de charge et de **chaos** (coupure du disque simulée en plein transfert) via GitHub Actions.
- Mise à jour automatique du binaire facultative, désactivée par défaut sur serveur.

## 12. Exécutable et déploiement

**Fichiers produits** : `c02hub.exe` (Windows x64), `c02hub` (Linux x64 et arm64), image Docker `ghcr.io/drfrrobin/c02-hub`. Configuration unique `c02hub.toml`, créée au premier lancement avec des valeurs sûres, plus variables d'environnement pour Docker.

**Commandes**

```
c02hub run              # démarre le serveur
c02hub init             # assistant : configuration, clé maîtresse, compte administrateur
c02hub tunnel setup     # assistant guidé Cloudflare (jeton, domaine)
c02hub install-service  # service Windows ou unité systemd
c02hub backup / restore # sauvegarde et restauration
c02hub vault check      # contrôle d'intégrité du coffre
c02hub admin ...        # gestion des comptes et quotas
c02hub doctor           # diagnostic (tunnel, bases, coffre, horloge, droits, espace disque)
```

**Cloudflare Tunnel**

1. Créer le tunnel (Zero Trust) et récupérer le jeton.
2. `c02hub tunnel setup` enregistre le jeton, chiffré, et vérifie la route.
3. Route publiée : `hub.c02.app` → `http://127.0.0.1:8787`.
4. Le Hub refuse de démarrer en écoute publique sans option explicite.

**Mode seul ou à plusieurs**

- **Seul (v1, par défaut)** : une instance, SQLite locale, coffre local.
- **Plusieurs instances (v1.5)** : plusieurs connecteurs `cloudflared` sur le même tunnel.
  - Comptes et index partagés via **PostgreSQL**, même schéma, choisi par configuration.
  - Le **coffre** passe sur un stockage partagé (NAS en réseau) ou sur le palier S3/R2, qui devient le coffre principal.
  - Les salons sont rattachés à une instance et retrouvés grâce à un préfixe du code, sans état partagé. La file d'attente est mutualisée via la base.
  - Si une instance tombe, ses joueurs se reconnectent ailleurs ; les parties P2P déjà établies continuent.

## 13. Compatibilité et migration

1. Le Hub **reprend l'API actuelle** (`/api/register`, `/api/login`, `/api/health`, profil, XP) pour que le jeu web fonctionne sans changement le jour de la bascule.
2. `c02hub import-node` migre `/data/c02.sqlite` : comptes, hachages scrypt (revalorisés en Argon2id à la prochaine connexion), XP et statistiques rattachées à `c02-naval`.
3. Bascule : déployer le Hub à côté de l'ancien, tester sur un sous-domaine, puis changer la route du tunnel. L'ancien conteneur reste disponible pour un retour arrière.
4. Les sauvegardes locales existantes (stockage du navigateur) sont **envoyées une fois** au cloud, après confirmation du joueur.

## 14. Intégration des jeux

### 14.1 C02 Naval

- Remplacer le service public PeerJS par le SDK du Hub (signalisation WebRTC via `/ws`). PeerJS reste en secours pendant la transition.
- Salon privé, adversaire aléatoire et reprise après coupure passent par le Hub, avec la cascade de §5.4.
- **Anti-triche T2** (engagement-révélation de la flotte) : à implémenter côté client, vérifié par l'adversaire et par le Hub. L'anti-triche actuel par pair-à-pair est conservé comme première ligne.
- Sauvegardes : profil, préférences, parties en cours si le jeu le propose, 3 slots.
- Version Windows (Rust) : même protocole via le client Rust.
- Aucun changement des règles du jeu.

### 14.2 Zombie Survivor

- **À analyser avant chiffrage** : solo, coopératif ou compétitif ? Technologie ? Trafic temps réel ? Ce dépôt n'a pas encore été audité.
- Hypothèse de départ : jeu solo à progression avec classement, donc **sauvegardes cloud**, **score signé** et **graine signée** (T0, T1, T3 par échantillonnage). S'il est coopératif en temps réel, cascade A à C préférée, relais du Hub limité.

### 14.3 Jeu tiers

- Guide « Ajouter un jeu en 30 minutes » : manifeste, import du SDK, appels d'API, choix des paliers anti-triche et du modèle de conflit de sauvegarde.

## 15. Plan de réalisation

| Phase | Contenu | Livrable |
|---|---|---|
| 0 | Audit de Zombie Survivor, validation de ce cahier, choix du nom de domaine | Cahier v1.0 validé |
| 1 | Cœur : configuration, `core.db`, comptes, sessions, API compatible avec l'existant, import Node | Hub 0.1 remplaçant du serveur Node |
| 2 | WebSocket, salons, file aléatoire, SDK JS, sonde de connectivité, paliers A, B, D | C02 Naval sur le Hub |
| 3 | **Cloud Saves** : `saves.db`, coffre, écriture atomique, versions, quotas, transferts reprenables, ramasse-miettes | Sauvegardes cloud |
| 4 | TOTP, WebAuthn, clés d'appareil, sessions par appareil, administration locale | Comptes sécurisés |
| 5 | Anti-triche T0, T1, T2 ; super-pairs (palier C) ; réputation | Parties fiables |
| 6 | Manifestes multi-jeux complets, rechargement à chaud, intégration de Zombie Survivor, T3 | 2 jeux actifs |
| 7 | Service Windows/Linux, assistant de tunnel, sauvegardes automatiques, palier froid, métriques, tests de charge et de chaos | Version 1.0 |
| 8 | Multi-instances et PostgreSQL | Version 1.5 |
| 9 | Amis, chat, canaux | Version 2 |

## 16. Critères d'acceptation de la v1.0

- `c02hub run` démarre avec un fichier de configuration généré ; binaire ≤ 15 Mo.
- Aucun port ouvert sur la box ; l'IP n'apparaît dans aucune réponse ni aucun journal.
- C02 Naval et Zombie Survivor fonctionnent simultanément avec un même compte.
- **Sauvegardes** :
  - un conflit entre deux appareils produit un `409` et **aucune version n'est perdue** ;
  - un arrêt brutal pendant l'écriture (test de chaos) ne laisse jamais une version pointant vers un blob absent ;
  - les quotas du manifeste sont appliqués (slots, taille, total, cadence) avec des codes d'erreur explicites ;
  - la restauration complète d'une sauvegarde de l'ensemble (bases + coffre) est vérifiée.
- **Connectivité** : sur un jeu de tests simulant NAT ouvert, NAT symétrique et UDP bloqué, chaque paire de joueurs trouve une voie ; la bascule en cours de partie fonctionne.
- **Anti-triche** : un client modifié qui envoie un résultat absurde, un faux « touché » ou un résultat non co-signé n'obtient aucun XP.
- 5 000 connexions simulées sous 250 Mo, sans perte de message de signalisation.
- Arrêt forcé (kill) : aucune donnée perdue ; les clients se reconnectent seuls en moins de 30 s.
- TOTP et une clé de sécurité fonctionnent de bout en bout.
- Migration des comptes existants vérifiée sans perte.

## 17. Risques et points ouverts

| Risque | Réponse |
|---|---|
| Le P2P échoue derrière certains réseaux | Cascade A à E et oracle de connectivité ; TURN externe optionnel |
| Le relais du Hub devient coûteux ou contraire aux conditions du tunnel | Chiffrement de bout en bout, plafonds par jeu, super-pairs, TURN externe |
| Les sauvegardes saturent le disque | Quotas par jeu, volume global plafonné, mode lecture seule sous un seuil, palier froid |
| Chiffrement de bout en bout et validation serveur incompatibles | Choix par jeu (`encryption`), validation limitée aux métadonnées signées |
| Perte de la clé de chiffrement par le joueur | Phrase de récupération affichée une fois ; avertissement explicite ; chiffrement facultatif par défaut |
| Point de défaillance unique à la maison | Sauvegardes quotidiennes + palier froid ; multi-instances et VPS de secours en v1.5 |
| IP exposée aux adversaires en P2P | IP masquée par défaut pour les parties aléatoires |
| Zombie Survivor non audité | Phase 0 obligatoire |
| Faux positifs de l'anti-triche | Sanctions réversibles, journaux conservés, tableau des litiges en administration |
| Complexité de l'ensemble | Livraison par phases ; chaque palier optionnel désactivable par configuration |

## 18. Questions ouvertes

1. Quel **nom de domaine** pour le Hub (aujourd'hui `api.c02.app`) ?
2. **Zombie Survivor** : solo, coopératif ou compétitif, et sur quelle technologie ?
3. **E-mail facultatif** pour la récupération de compte : accepté ?
4. **TURN embarqué** (port UDP ouvert) : exclu définitivement, ou conservé en option ?
5. **Taille de communauté** visée à 6 mois (dizaines, centaines, milliers) ?
6. **Modération** (pseudos interdits, signalement) avant ouverture à des inconnus ?
7. **Palier froid** S3 / Cloudflare R2 : souhaité dès la v1 ou plus tard ?
8. **Chiffrement de bout en bout** des sauvegardes : activé par défaut ou à la demande du joueur ?
9. Le Hub reste-t-il dans ce dépôt (`/hub`) ou devient-il un dépôt dédié ?

# Cahier des charges — C02 Hub (serveur de jeux multi-jeux)

Version 0.1 — 2026-11-18 — statut : **à valider**

## 1. Contexte et objectif

Aujourd'hui, `server/` (Node.js) gère uniquement les comptes de C02 Naval (voir `docs/SERVEUR.md`). Les parties passent en pair-à-pair via PeerJS, et l'API tourne dans un conteneur sur un NAS TrueNAS derrière un Cloudflare Tunnel.

**Objectif** : remplacer ce serveur par **un seul exécutable**, le « C02 Hub ». Il héberge plusieurs jeux en même temps (C02 Naval, [Zombie-Survivor-By-C02](https://github.com/Ghoster-001/Zombie-Survivor-By-C02), et d'autres plus tard). Il apporte aux jeux un socle commun :

- comptes uniques pour tous les jeux ;
- mise en relation des joueurs (« signalisation ») ;
- profils, XP et statistiques par jeu ;
- mise à disposition d'un chat ou d'amis plus tard, sans refonte.

**Principes directeurs**

1. **Charge minimale.** Le serveur ne porte pas le trafic de jeu. Il authentifie, met en relation et stocke.
2. **Fiable.** Aucune perte de données, redémarrage transparent, comportement prévisible sous charge.
3. **Aucun port ouvert sur la box.** Tout passe par un Cloudflare Tunnel sortant. L'IP de la box n'est jamais publiée.
4. **Un seul fichier à déployer**, sans dépendance système.
5. **Extensible par modules.** Ajouter un jeu ne demande aucune modification du cœur.

## 2. Décisions déjà prises

| Sujet | Décision |
|---|---|
| Topologie multijoueur | **P2P + relais de secours** (désactivable) |
| Langage | **Rust** (le plus optimisé en RAM et en CPU), binaire unique |
| Comptes | Pseudo + mot de passe, **2FA TOTP (application d'authentification)** et **clés de sécurité WebAuthn / passkeys** |
| Hébergement | PC ou NAS à la maison + **Cloudflare Tunnel** |
| Mode d'exécution | Une instance seule, ou plusieurs instances (voir §10) |

## 3. Périmètre

**Dans le périmètre**

- Exécutable serveur (Windows x64 et Linux x64/arm64, avec image Docker pour le NAS).
- API HTTPS + WebSocket de signalisation.
- Système de modules de jeu (§6).
- Compte unique, profils, sessions, 2FA.
- Stockage de l'XP et des statistiques par jeu.
- Page d'administration locale.
- Adaptation de **C02 Naval** et de **Zombie Survivor** (§12).

**Hors périmètre (v1)**

- Hébergement du trafic de jeu en temps réel (sauf relais de secours).
- Chat, amis et messagerie type Discord : prévus en v2, le modèle de données les anticipe (§7).
- Paiements, boutique.

## 4. Architecture

```
Client (navigateur / .exe) ──HTTPS+WSS──► Cloudflare ──Tunnel sortant──► cloudflared ──► C02 Hub (127.0.0.1:8787)
        ▲                                                                                │
        └──────── P2P direct (WebRTC / UDP) entre joueurs ◄── échange de candidats ──────┘
                         └─ si impossible : relais de secours (TURN ou relais WebSocket du Hub)
```

- **Runtime** : Rust, `tokio` (asynchrone, multi-thread), `axum` pour HTTP et WebSocket.
- **Écoute locale uniquement** (`127.0.0.1`) : seul le tunnel atteint le Hub.
- **`cloudflared`** : processus séparé. Le Hub peut le lancer et le surveiller lui-même (option `tunnel.mode = "managed"`), ou être exposé par un `cloudflared` déjà présent (`"external"`).
- **Base de données** : SQLite en mode WAL (embarqué, aucune installation). L'accès passe par un petit pool de connexions. Les écritures sont regroupées par lots.
- **État en mémoire** : salons et présence uniquement. Il est reconstruit au redémarrage, rien de critique n'y vit.

### 4.1 Pourquoi le tunnel Cloudflare convient

- Le tunnel gratuit transporte HTTP(S) et WebSocket. C'est suffisant, car le Hub fait de la signalisation et pas du transport de jeu.
- Le trafic de jeu direct (WebRTC) ne passe jamais par Cloudflare. Il sort directement de chaque joueur.
- Ton IP reste cachée : seul le domaine Cloudflare est public. Les dépendances à un DHCP ne comptent pas, puisque le tunnel est sortant.
- Limite à connaître : un serveur de jeu TCP/UDP brut ne pourrait **pas** passer par le tunnel gratuit. Le choix du P2P évite ce problème.

## 5. Fonctionnalités du socle

### 5.1 Comptes et authentification

- Inscription avec pseudo unique (3–20 caractères, insensible à la casse) et mot de passe.
- Hachage **Argon2id**. Les anciens comptes scrypt du serveur Node sont migrés à la première connexion.
- **2FA TOTP** : activation avec QR code, 10 codes de secours à usage unique.
- **Clés de sécurité / passkeys WebAuthn** : plusieurs par compte, nommables et révocables.
- Politique par compte : mot de passe seul, mot de passe + 2FA, ou passkey seule.
- Sessions : jeton aléatoire de 256 bits, seul son hachage est en base. Les sessions sont listées et révocables par appareil. Elles durent 60 jours, avec renouvellement glissant.
- Anti-force brute : verrouillage progressif par compte et limitation de débit par IP (en-tête `CF-Connecting-IP`, de confiance uniquement via le tunnel).
- Réponses identiques pour « pseudo inconnu » et « mauvais mot de passe ».
- Changement de mot de passe, suppression de compte et export des données (conformité RGPD).
- **Récupération** : sans e-mail, via les codes de secours ou une passkey. Un e-mail facultatif pourra être ajouté plus tard.
- Mode invité : jeton éphémère, sans stockage de données.

### 5.2 Profils et progression

- Profil global : pseudo, avatar, date de création.
- Données **par jeu** : XP, niveau, statistiques, succès. Stockées sous forme clé/valeur typée, avec un schéma déclaré par chaque module.
- **Le client n'envoie jamais l'XP.** Il envoie un résultat de partie. Le module du jeu le valide (vraisemblance, délai minimal) et calcule la récompense.

### 5.3 Mise en relation (signalisation)

- Le client ouvre un WebSocket authentifié sur `/ws`.
- **Salon privé** : code de 6 caractères, avec lien d'invitation.
- **Adversaire aléatoire** : file d'attente par jeu et par mode, avec délai avant repli sur un bot (réglable par jeu).
- **Échange de connexion** : le Hub relaie les offres et réponses WebRTC (SDP) et les candidats ICE entre pairs. Le Hub ne transmet l'adresse IP d'un joueur qu'à ses partenaires de partie, jamais en liste publique.
- Option « **masquer mon IP** » par joueur : la connexion passe alors par le relais de secours.
- Annonce de présence, reprise après coupure (le joueur retrouve son salon pendant 60 s) et abandon détecté.

### 5.4 Relais de secours

- **Niveau 1 : STUN.** Le Hub indique un serveur STUN public (configurable).
- **Niveau 2 : TURN.** Il s'agit soit d'un relais externe configuré, soit d'un TURN embarqué (fonction facultative, désactivée par défaut). Il est exposé hors Cloudflare, donc ouvrir un port est nécessaire, ce qui est contraire à l'objectif. Il reste donc **optionnel**.
- **Niveau 3 : relais WebSocket du Hub** (par défaut). Il fonctionne dans le tunnel, sans port ouvert. Il est limité en débit et en nombre de salons par jeu, car il devient le goulot d'étranglement. Il convient aux jeux au tour par tour (Naval) et aux petites parties. Il n'est pas adapté à du temps réel intense.
- La politique est choisie par jeu dans son manifeste : `p2p_only`, `p2p_then_relay` ou `relay_only`.

### 5.5 Administration

- Console locale `http://127.0.0.1:PORT/admin`, protégée par un compte administrateur et inaccessible via le tunnel (route liée à l'interface locale ou à un second port).
- Fonctions : liste des comptes, blocage, réinitialisation de 2FA, jeux actifs, salons en cours, journaux, métriques, sauvegarde manuelle.
- Option en ligne de commande équivalente (`c02hub admin ...`).

## 6. Système multi-jeux (modules)

Chaque jeu est un **module** décrit par un manifeste. Ajouter un jeu revient à ajouter un manifeste, et éventuellement du code de validation.

```toml
# games/c02-naval/game.toml
id            = "c02-naval"
name          = "C02 Naval"
min_players   = 2
max_players   = 2
transport     = "p2p_then_relay"
modes         = ["random", "private"]
bot_fallback_seconds = 45
allowed_origins = ["https://drfrrobin.github.io"]
[stats]
xp = "int"
wins = "int"
losses = "int"
[validation]
min_game_seconds = 20
```

- **Niveau 1 : déclaratif.** Le manifeste suffit pour la plupart des jeux : joueurs, modes, transport, statistiques.
- **Niveau 2 : validation par module.** Une logique de validation de fin de partie peut être fournie sous forme de module **WebAssembly** chargé à chaud (`wasmtime`). Il est isolé et sans accès réseau ni disque. Un module défaillant ne peut pas faire tomber le Hub. Un jeu dont le module est absent utilise la validation générique (délai minimal, plafonds).
- **Isolation entre jeux** : espaces de noms distincts pour les salons, les files et les statistiques. Quotas par jeu (salons simultanés, débit du relais). Un jeu peut être mis hors ligne sans toucher aux autres.
- **Chargement à chaud** : les manifestes et modules se rechargent via l'administration, sans redémarrage ni coupure des parties en cours.
- **Contrat de protocole** : tous les messages portent `{ v, game, type }`. Les versions sont négociées et un client trop ancien reçoit une réponse « mise à jour requise », comme dans la logique actuelle de C02 Naval.
- **SDK client** (JavaScript, ~5 Ko) : `hub.login()`, `hub.createRoom()`, `hub.join(code)`, `hub.findMatch()`, `hub.reportResult()`, et une interface commune à PeerJS/WebRTC. Un client Rust équivalent sert à la version Windows de C02 Naval.

## 7. Modèle de données (SQLite)

| Table | Contenu |
|---|---|
| `users` | id, pseudo, pseudo_norm (unique), mot de passe Argon2id, statut, création |
| `user_totp`, `user_recovery_codes` | secret TOTP chiffré, codes de secours hachés |
| `user_webauthn` | clé publique, compteur, nom, dernière utilisation |
| `sessions` | hash du jeton, user, appareil, expiration |
| `game_profiles` | (user, game) → JSON de statistiques, XP, niveau |
| `match_results` | partie, jeu, participants, résultat, horodatage (journal, base de l'anti-triche) |
| `audit_log` | connexions, échecs, actions d'administration |
| *v2 :* `friends`, `channels`, `messages` | prévus mais non créés en v1, pour accueillir un chat de type Discord |

- Migrations versionnées et appliquées automatiquement au démarrage, avec sauvegarde préalable.
- Les secrets (TOTP) sont chiffrés avec une clé maîtresse hors base (`C02_MASTER_KEY` ou fichier).

## 8. Exigences non fonctionnelles

### 8.1 Performance (cibles à mesurer)

| Indicateur | Cible |
|---|---|
| Mémoire au repos | ≤ 30 Mo |
| Mémoire avec 5 000 connexions WebSocket | ≤ 250 Mo |
| CPU au repos | ≈ 0 % |
| Taille du binaire | ≤ 25 Mo |
| Latence de signalisation (Hub → pair), hors réseau | ≤ 20 ms au 99e centile |
| Capacité visée sur un mini-PC / NAS modeste | ≥ 5 000 joueurs connectés, ≥ 100 requêtes/s |

- Messages WebSocket compacts (binaire ou JSON court), pas de diffusion inutile.
- Aucune allocation par message sur le chemin chaud, journalisation asynchrone.
- Un banc d'essai de charge (client simulé) est livré avec le projet.

### 8.2 Fiabilité

- Redémarrage automatique (service systemd, service Windows ou politique Docker `restart`).
- Arrêt propre sur signal : les écritures en cours sont terminées, les clients sont prévenus et se reconnectent seuls (délai exponentiel avec variation aléatoire).
- Sauvegarde automatique quotidienne de la base (API `backup` de SQLite), avec rotation de 14 jours. Une restauration est documentée et testée.
- Point de contrôle `/api/health` (vivacité) et `/api/ready` (base joignable).
- Aucune perte d'XP : un résultat n'est accepté qu'une fois (identifiant de partie unique, idempotence).
- Limites partout : taille des corps de requête, nombre de messages par seconde et par connexion, nombre de salons par compte.

### 8.3 Sécurité

- HTTPS terminé par Cloudflare. Le Hub n'écoute qu'en local, avec en-têtes de sécurité (CSP, HSTS fourni par Cloudflare, `X-Content-Type-Options`).
- CORS restreint aux origines déclarées dans les manifestes.
- Entrées validées strictement, aucune requête SQL concaténée, journaux sans secret.
- Dépendances auditées (`cargo audit`, `cargo deny`) à chaque compilation. Compilation reproductible et binaires signés avec sommes SHA-256 publiées.
- Aucune IP de joueur n'est conservée au-delà de la limitation de débit (en mémoire, 24 h). Politique de confidentialité à rédiger.
- Cloudflare WAF : limitation de débit sur `/api/login`, `/api/register` et les routes WebAuthn.

### 8.4 Observabilité

- Journaux structurés (JSON) avec niveaux, rotation.
- Métriques au format Prometheus sur `/metrics` (accès local) : connexions, salons, files, latences, erreurs, taille de la base.

### 8.5 Maintenabilité

- Cœur et modules de jeu séparés. Tests unitaires, tests d'intégration (clients simulés) et tests de charge exécutés par GitHub Actions.
- Mise à jour automatique du binaire facultative (même principe que le `.exe` Windows : vérification des versions GitHub, téléchargement et remplacement), désactivée par défaut sur serveur.

## 9. Exécutable et déploiement

**Fichiers produits**

- `c02hub.exe` (Windows x64), `c02hub` (Linux x64 et arm64), image Docker `ghcr.io/drfrrobin/c02-hub`.
- Une configuration unique `c02hub.toml` (créée au premier lancement avec des valeurs sûres), plus les variables d'environnement pour Docker.

**Commandes**

```
c02hub run              # démarre le serveur
c02hub init             # assistant : config, clé maîtresse, compte administrateur
c02hub tunnel setup     # assistant guidé Cloudflare (jeton, domaine)
c02hub install-service  # service Windows ou unité systemd
c02hub backup / restore # sauvegarde et restauration
c02hub admin ...        # gestion des comptes en ligne de commande
c02hub doctor           # diagnostic (ports, tunnel, base, horloge, droits)
```

**Cloudflare Tunnel**

1. Créer le tunnel (tableau de bord Zero Trust) et récupérer le jeton.
2. `c02hub tunnel setup` enregistre le jeton, chiffré, et vérifie la route.
3. Route publiée : `hub.c02.app` → `http://127.0.0.1:8787`.
4. Le Hub refuse de démarrer en écoute publique sans option explicite.

## 10. Exécution « seul » ou « à plusieurs »

- **Mode seul (v1, par défaut)** : une instance, SQLite locale. Convient jusqu'à plusieurs milliers de joueurs.
- **Mode multi-instances (v1.5)** : plusieurs Hubs derrière le même tunnel (plusieurs connecteurs `cloudflared` sur un même tunnel, pour la redondance).
  - Les comptes et sessions sont partagés via une base commune : passage de SQLite à **PostgreSQL** (même schéma, choisi par la configuration).
  - Les salons sont rattachés à une instance. Un salon est retrouvé grâce à un préfixe du code, évitant tout état partagé. La file d'attente aléatoire est mutualisée via la base.
  - Si une instance tombe, ses joueurs se reconnectent à une autre et perdent leurs salons en cours (les parties P2P déjà établies continuent).
- Le choix entre les deux modes se fait uniquement par configuration, sans changer d'exécutable.

## 11. Compatibilité avec l'existant (migration)

1. Le Hub **reprend l'API actuelle** (`/api/register`, `/api/login`, `/api/health`, profil et XP) pour que le jeu web actuel fonctionne sans changement le jour du basculement.
2. Un outil `c02hub import-node` migre `/data/c02.sqlite` du serveur Node : comptes, hachages scrypt conservés et revalorisés en Argon2id à la prochaine connexion, XP et statistiques rattachées au jeu `c02-naval`.
3. Bascule : déployer le Hub à côté de l'ancien, tester via un sous-domaine, puis changer la route du tunnel. L'ancien conteneur reste disponible pour un retour arrière.

## 12. Intégration des jeux

### 12.1 C02 Naval

- Remplacer l'appel PeerJS public par le SDK du Hub (signalisation WebRTC via `/ws`), en gardant PeerJS en secours pendant la transition.
- Salon privé, adversaire aléatoire et reprise après coupure passent par le Hub.
- Anti-triche existant conservé côté client pair-à-pair. La validation du résultat est faite par le module `c02-naval` (XP recalculée).
- Version Windows (Rust) : même protocole, via le client Rust.
- Aucun changement des règles du jeu.

### 12.2 Zombie Survivor

- À analyser avant le chiffrage : ce dépôt n'a pas été audité. Il faut déterminer s'il est solo ou multijoueur, sa technologie et le volume de trafic temps réel.
- Hypothèse : si le jeu est **coopératif en temps réel**, la politique `p2p_then_relay` s'applique, sans relais pour la boucle de jeu. Il faudra alors décider si le Hub porte un état de salle.
- Livrables : manifeste `zombie-survivor`, comptes partagés, classements et scores validés.

### 12.3 Jeu tiers générique

- Documentation « Ajouter un jeu en 30 minutes » : manifeste, import du SDK, appels d'API.

## 13. Plan de réalisation

| Phase | Contenu | Livrable |
|---|---|---|
| 0 | Audit de Zombie Survivor, validation de ce cahier, choix du nom de domaine | Cahier validé |
| 1 | Cœur : config, SQLite, comptes, sessions, API compatible avec l'existant | Hub 0.1, remplaçant du serveur Node |
| 2 | WebSocket, salons, file aléatoire, SDK JS, relais WebSocket | C02 Naval sur le Hub |
| 3 | TOTP, WebAuthn, sessions par appareil, administration locale | Comptes sécurisés |
| 4 | Manifestes multi-jeux, quotas, validation WASM, intégration de Zombie Survivor | 2 jeux actifs |
| 5 | Service Windows/Linux, assistant de tunnel, sauvegardes, métriques, tests de charge | Version 1.0 |
| 6 | Multi-instances et PostgreSQL | Version 1.5 |
| 7 | Amis, chat, canaux | Version 2 |

## 14. Critères d'acceptation de la v1.0

- Un seul exécutable démarre avec `c02hub run` et un fichier de configuration généré.
- Aucun port ouvert sur la box : un scan externe ne voit rien, et l'IP de la box n'apparaît dans aucune réponse.
- C02 Naval et Zombie Survivor fonctionnent simultanément avec un même compte.
- Le Hub reste sous 250 Mo avec 5 000 connexions simulées, sans perte de messages de signalisation.
- Redémarrage forcé (kill) : aucune donnée perdue, les clients se reconnectent seuls en moins de 30 s.
- TOTP et une clé de sécurité fonctionnent de bout en bout.
- Migration des comptes existants vérifiée sans perte.
- Restauration d'une sauvegarde testée.

## 15. Risques et points ouverts

| Risque | Réponse |
|---|---|
| Le P2P échoue derrière certains NAT | Relais WebSocket en repli, TURN externe optionnel |
| Le relais WebSocket devient coûteux si les joueurs l'utilisent massivement | Quotas par jeu, débit limité, bascule vers TURN externe |
| Le tunnel Cloudflare gratuit est soumis aux conditions de Cloudflare (usage de gros volumes de médias interdit) | Trafic limité à la signalisation, documents d'usage vérifiés |
| Point de défaillance unique à la maison (coupure de courant, box) | Mode multi-instances et VPS de secours en v1.5 |
| IP exposée aux adversaires en P2P | Option « masquer mon IP » qui force le relais |
| Zombie Survivor non audité | Phase 0 obligatoire |
| Complexité WebAuthn / WASM | Livrés en phases 3 et 4, après le socle |

## 16. Questions ouvertes

1. Quel **nom de domaine** utiliser pour le Hub (actuellement `api.c02.app`) ?
2. Zombie Survivor est-il solo, coopératif ou compétitif, et quelle technologie utilise-t-il (Canvas/JS, moteur, etc.) ?
3. Accepte-t-on un **e-mail facultatif** pour la récupération de compte ?
4. Le **TURN embarqué** (port UDP ouvert) est-il définitivement exclu, ou le garde-t-on en option ?
5. Quelle **taille de communauté** vises-tu à 6 mois (dizaines, centaines, milliers de joueurs) ?
6. Faut-il une **vérification d'âge** ou des règles de modération (pseudos interdits, signalement) avant d'ouvrir à des inconnus ?
7. Le projet reste-t-il dans ce dépôt (`/hub`) ou devient-il un dépôt dédié ?

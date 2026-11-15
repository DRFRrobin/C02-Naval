# Serveur de comptes (TrueNAS SCALE + Cloudflare Tunnel)

L'API (`server/`) gère les comptes : **pseudo unique + mot de passe**, profil, XP et statistiques synchronisés entre appareils. Elle tourne dans un conteneur sur ton NAS, et le jeu continue d'être servi par GitHub Pages. Les parties restent en pair-à-pair (PeerJS + TURN).

```
Navigateur ── HTTPS ──► https://api.c02.app ── Cloudflare Tunnel ──► NAS : conteneur c02-naval-api (port 8787)
                                                                         └─ /data/c02.sqlite (volume)
```

## 1. Publier l'image (une seule fois)
1. Fusionne la PR : le workflow `api-image` construit et publie `ghcr.io/drfrrobin/c02-naval-api:latest`.
2. GitHub → ton profil → **Packages** → `c02-naval-api` → *Package settings* → **Change visibility → Public** (pour que TrueNAS puisse la télécharger sans identifiants ; l'image ne contient aucun secret).

## 2. Créer l'application dans TrueNAS (25.10)
1. **Apps → Discover Apps → ⋮ → Install via YAML**.
2. Nom : `c02-naval-api`. Colle le contenu de [`docker-compose.nas.yml`](../docker-compose.nas.yml).
3. Adapte `ALLOWED_ORIGINS` si ton jeu est servi ailleurs (plusieurs adresses : sépare par des virgules, sans `/` final).
4. Enregistre. Vérifie dans le navigateur de ton réseau : `http://IP-DU-NAS:8787/api/health` doit répondre `{"ok":true}`.

> Le volume `c02-data` contient la base. Pour qu'il soit sauvegardé comme le reste, remplace-le par un dossier d'un dataset (`/mnt/POOL/c02-naval:/data`, dossier lisible/écrivable par l'utilisateur 1000 ou « node »).

## 3. Exposer l'API avec le tunnel Cloudflare
Dans Cloudflare Zero Trust → **Networks → Tunnels →** ton tunnel → **Published application routes → Add** :
- **Sous-domaine / domaine :** `api` / `c02.app`
- **Service :** `HTTP` → `IP-DU-NAS:8787` (ou le nom du conteneur si `cloudflared` est dans le même réseau Docker)

Le port 8787 n'a **pas** besoin d'être ouvert sur ta box. Ne l'expose pas en dehors de ton réseau local : `TRUST_PROXY=1` fait confiance à l'en-tête `CF-Connecting-IP` pour la limitation de débit, ce qui n'est sûr que si seul le tunnel atteint l'API.

Recommandé côté Cloudflare : **Security → WAF → Rate limiting** sur `api.c02.app/api/login` et `/api/register` (par exemple 10 requêtes / minute / IP).

## 4. Brancher le jeu
Le jeu appelle `https://api.c02.app` par défaut. Pour utiliser une autre adresse, ajoute avant le script principal d'`index.html` : `<script>window.C02_API='https://mon-api.exemple'</script>`. Une valeur vide désactive les comptes en ligne (retour au pseudo local).

## Sécurité en bref
- Mots de passe : `scrypt` (N=32768), sel aléatoire, comparaison à temps constant ; jamais stockés ni journalisés en clair.
- Verrouillage 15 min après 5 échecs sur un compte, limitation de débit par adresse IP, réponse identique pour « pseudo inconnu » et « mauvais mot de passe ».
- Jetons de session aléatoires (256 bits), seul leur hachage est en base, valables 60 jours, révoqués à la déconnexion et au changement de mot de passe.
- L'XP n'est jamais envoyée par le client : le serveur la recalcule à partir du résultat de la partie, avec contrôles de vraisemblance et délai minimum entre deux résultats.
- CORS restreint aux origines listées, corps de requête limité à 16 Ko, conteneur sans privilèges (système de fichiers en lecture seule, sans capacités).
- Il n'y a pas de récupération de mot de passe : sans e-mail, un mot de passe perdu signifie un compte perdu.

## Sauvegarde
La base est le fichier `/data/c02.sqlite` (mode WAL : copie aussi `-wal` et `-shm`, ou utilise `sqlite3 c02.sqlite ".backup sauvegarde.sqlite"`). Planifie une tâche de réplication/snapshot TrueNAS du dataset.

## Mettre à jour
Chaque fusion sur `main` qui touche `server/` publie une nouvelle image. Dans TrueNAS : **Apps → c02-naval-api → Update** (ou redémarre l'application pour retélécharger `latest`).

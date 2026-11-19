# ⚓ C02 Naval — version Windows native (Rust)

Le jeu C02 Naval complet, réécrit en Rust sous forme d'application Windows 11 x64 native (un seul `.exe`, sans navigateur, sans installation).

## ⬇️ Téléchargement

- **[Télécharger la dernière version (C02-Naval-windows-x64.exe)](https://github.com/DRFRrobin/C02-Naval/releases/latest/download/C02-Naval-windows-x64.exe)**
- **[Voir toutes les versions](https://github.com/DRFRrobin/C02-Naval/releases)** (la plus récente en haut)

L'exécutable n'est pas signé : si Windows SmartScreen affiche un avertissement, cliquer sur « Informations complémentaires » puis « Exécuter quand même ».

### Versions

La plus récente en premier :

| Version | Date | Changements |
|---|---|---|
| [**1.2.4**](https://github.com/DRFRrobin/C02-Naval/releases/tag/v1.2.4-windows) (dernière) | 2026-11-18 | Parité avec la version web 1.2.4 : comptes en ligne, règles personnalisables, anti-triche, revanche, reprise après coupure, 7 thèmes dont Rétro, 6 langues, image de partage |
| [1.2.0](https://github.com/DRFRrobin/C02-Naval/releases/tag/v1.2.0-windows) | 2026-11-15 | Mise à jour obligatoire depuis GitHub à chaque lancement |
| [1.1.0](https://github.com/DRFRrobin/C02-Naval/releases/tag/v1.1.0-windows) | 2026-11-15 | Première version Windows en Rust |

> La version 1.1.0 ne peut pas se mettre à jour toute seule : la remplacer une fois à la main par la dernière version.

## Mises à jour automatiques (obligatoires)

À chaque lancement, le jeu consulte les [versions publiées sur GitHub](https://github.com/DRFRrobin/C02-Naval/releases) (tags se terminant par `-windows`, fichier `.exe` joint).

- Pas de version plus récente : le jeu démarre normalement.
- Version plus récente : la mise à jour est **obligatoire**. Le jeu la télécharge, remplace son propre `.exe` et redémarre, sans possibilité de jouer à l'ancienne version.
- Pas d'Internet (ou GitHub ne répond pas) : le jeu démarre avec la version actuelle.
- Si le remplacement échoue (par exemple dans un dossier protégé en écriture comme `Program Files`), un lien vers la page des versions est affiché.

Pour publier une mise à jour : augmenter `version` dans `Cargo.toml`, ajouter une ligne en haut du tableau **Versions**, puis pousser le tag `vX.Y.Z-windows` : le workflow `rust-windows` compile, teste et joint `C02-Naval-windows-x64.exe` à la release.

## Fonctionnalités (identiques à la version web 1.2.4)

- **Comptes** : compte en ligne (pseudo unique + mot de passe, même serveur que le site : profil et XP partagés) ou pseudo local sans mot de passe, ou mode invité. Avatar : emoji (mêmes 55 que le site) ou photo recadrée en 160 × 160.
- **Tableau de bord** : barre du haut (avatar, niveau, barre d'XP, grade), onglets Jouer / Statistiques / Règles / Paramètres / Déconnexion.
- **4 modes** : adversaire aléatoire (bot de secours au bout de 45 s, 20 s par coup), partie contre le bot tactique (3 niveaux), rejoindre avec un code, héberger un salon privé (code de 6 caractères, copie du code ou du lien d'invitation).
- **Règles personnalisables** : grille de 6 × 6 à 15 × 15, flotte libre (jusqu'à 10 bateaux, 40 % de la grille), Radar / Frappe en croix / Bouclier activables, chronomètre de tour. Elles s'appliquent aux parties « Hôte » et contre le bot ; en rejoignant un ami, ce sont les règles de l'hôte.
- **Placement** : aperçu puis validation, glisser-déposer, rotation (touche R ou clic droit), aléatoire, effacer, raccourcis Entrée / Suppr. Avec un chronomètre, 90 s maximum : les bateaux restants sont posés au hasard.
- **Partie** : tir, **Radar** (2 × 2) et **Frappe en croix** (5 cases, un premier appui vise, un second confirme), **Bouclier**, choix d'une amélioration après chaque bateau coulé, journal, dernier tir ennemi mis en évidence, tir automatique à la fin du chronomètre.
- **Anti-triche** : empreinte SHA-256 de la grille au moment du « prêt », grille dévoilée et vérifiée en fin de partie, détection de réponses impossibles.
- **Fin de partie** : détail de l'XP, revanche (le premier tir alterne), bateaux adverses non trouvés, **image de partage 1080 × 1350** identique à celle du site (enregistrée dans Images et copiée dans le presse-papiers).
- **Reprise après coupure** : 45 s pour reprendre la partie si la liaison tombe, comme sur le site.
- **Statistiques de carrière**, XP et niveaux (Mousse → Amiral), XP divisée par deux contre le bot et hors règles standard.
- **Paramètres** : effets sonores synthétisés, **6 langues** (français, anglais, portugais, espagnol, allemand, italien), **7 thèmes** dont Rétro arcade et Rétro Noël (polices pixel, par défaut en décembre), bot activable et niveau, relais TURN facultatif, avatar, mot de passe (compte en ligne), suppression du compte.
- Garde l'écran allumé pendant une partie en ligne ; réduire la fenêtre = « absent » pour l'adversaire (2 min).

## Compatible avec la version web

Le réseau reproduit exactement PeerJS 1.5 : mise en relation via le service PeerJS public, lien WebRTC direct, messages BinaryPack découpés en morceaux de 16 300 octets. **Un joueur sur PC peut affronter un joueur dans le navigateur** (salon privé, rejoindre, adversaire aléatoire). Le bouton « Copier le lien » donne le lien d'invitation du site web.

> Compatibilité testée avec la version web 1.2.4 : une partie complète Rust ↔ navigateur (placement, tirs, améliorations, vérification anti-triche) se déroule normalement.

## Données

Comptes locaux, cache des comptes en ligne et paramètres : `%APPDATA%\C02 Naval\` (un fichier JSON par compte).
La variable d'environnement `C02NAVAL_DATA` change ce dossier (version portable sur clé USB, par exemple).
`C02NAVAL_DEBUG=1` écrit un journal réseau (`net.log`) dans ce dossier.
`C02NAVAL_NO_UPDATE=1` désactive la vérification des mises à jour (développement).

## Compilation

Prérequis : [Rust](https://rustup.rs) (stable, chaîne `x86_64-pc-windows-msvc`) et les outils de compilation C++ de Visual Studio.

```bash
cd rust
cargo build --release
```

L'exécutable : `rust/target/release/c02-naval.exe` (icône intégrée, sans fenêtre de console).

Tests : `cargo test --release` (format BinaryPack, empreinte de grille identique à celle du site, formule d'XP, validation des règles, textes des 6 langues, client des comptes en ligne, image de partage, bot à 3 niveaux).

## Organisation du code

| Fichier | Rôle |
|---|---|
| `src/main.rs` | Fenêtre, icône |
| `src/app.rs` | État et logique (comptes, mise en relation, placement, partie, chronomètres) |
| `src/ui.rs` | Écrans (egui) |
| `src/theme.rs`, `src/palettes.rs` | Thèmes, polices (Segoe UI, Bahnschrift, polices pixel), widgets personnalisés |
| `src/cloud.rs` | Client des comptes en ligne |
| `src/shareimg.rs` | Image de partage |
| `src/i18n.rs`, `src/i18n_data.rs` | Textes des 6 langues (générés depuis le site) |
| `src/emoji.rs` | Avatars emoji (Twemoji embarqués) |
| `src/game.rs` | Règles, résolution des tirs, bot tactique |
| `src/net.rs` | Signalisation PeerJS + WebRTC (crate `webrtc`) |
| `src/binarypack.rs` | Format BinaryPack de PeerJS |
| `src/accounts.rs` | Comptes, mots de passe, photos, paramètres |
| `src/sound.rs` | Synthèse sonore (cpal) |
| `src/i18n.rs` | Textes français / anglais |
| `src/update.rs` | Mise à jour obligatoire depuis les releases GitHub |
| `tools/` | Scripts de test (capture de fenêtre, clics simulés) |

## Crédits

Les emojis d'avatar sont des images [Twemoji](https://github.com/jdecked/twemoji) (© Twitter, Inc. et contributeurs), sous licence [CC-BY 4.0](https://creativecommons.org/licenses/by/4.0/).
Les polices pixel des thèmes rétro sont « Press Start 2P » et « VT323 » (SIL Open Font License 1.1).

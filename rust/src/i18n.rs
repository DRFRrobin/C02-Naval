//! Textes de l'interface (français / anglais).

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    Fr,
    En,
    Pt,
    Es,
    De,
    It,
}

impl Lang {
    pub const ALL: [Lang; 6] = [Lang::Fr, Lang::En, Lang::Pt, Lang::Es, Lang::De, Lang::It];

    /// Indice dans les tables de `i18n_data`.
    pub fn idx(self) -> usize {
        self as usize
    }

    /// Nom de la langue, écrit dans cette langue.
    pub fn label(self) -> &'static str {
        match self {
            Lang::Fr => "Français",
            Lang::En => "English",
            Lang::Pt => "Português",
            Lang::Es => "Español",
            Lang::De => "Deutsch",
            Lang::It => "Italiano",
        }
    }

    #[cfg_attr(windows, allow(dead_code))]
    fn from_code(c: &str) -> Option<Lang> {
        match c.get(..2)?.to_lowercase().as_str() {
            "fr" => Some(Lang::Fr),
            "en" => Some(Lang::En),
            "pt" => Some(Lang::Pt),
            "es" => Some(Lang::Es),
            "de" => Some(Lang::De),
            "it" => Some(Lang::It),
            _ => None,
        }
    }

    pub fn detect() -> Lang {
        // Langue de Windows (langue d'affichage), sinon variable LANG ; anglais pour une langue non gérée.
        #[cfg(windows)]
        {
            extern "system" {
                fn GetUserDefaultUILanguage() -> u16;
            }
            let id = unsafe { GetUserDefaultUILanguage() } & 0x3ff;
            return match id {
                0x0c => Lang::Fr,
                0x16 => Lang::Pt,
                0x0a => Lang::Es,
                0x07 => Lang::De,
                0x10 => Lang::It,
                _ => Lang::En,
            };
        }
        #[cfg(not(windows))]
        {
            std::env::var("LANG").ok().and_then(|l| Lang::from_code(&l)).unwrap_or(Lang::En)
        }
    }
}

pub fn pl(n: usize, one: &str, many: &str) -> String {
    format!("{} {}", n, if n > 1 { many } else { one })
}

/// Remplace `{clé}` par sa valeur.
pub fn fill(s: &str, p: &[(&str, String)]) -> String {
    let mut out = s.to_string();
    for (k, v) in p {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

pub fn tr(lang: Lang, key: &str) -> &'static str {
    let table = [crate::i18n_data::FR, crate::i18n_data::EN, crate::i18n_data::PT, crate::i18n_data::ES, crate::i18n_data::DE, crate::i18n_data::IT][lang.idx()];
    if let Some(s) = own(lang, key) {
        return s;
    }
    if let Ok(i) = table.binary_search_by(|(k, _)| (*k).cmp(key)) {
        return table[i].1;
    }
    let fr = lang == Lang::Fr;
    macro_rules! t {
        ($f:expr, $e:expr) => {
            if fr { $f } else { $e }
        };
    }
    match key {
        "tagline" => t!("Bataille navale à deux, sur PC ou téléphone.", "Two-player naval battle, on PC or phone."),
        "tabLogin" => t!("Connexion", "Log in"),
        "tabSignup" => t!("Créer un compte", "Sign up"),
        "account" => t!("Compte", "Account"),
        "password" => t!("Mot de passe", "Password"),
        "passwordConfirm" => t!("Confirmer le mot de passe", "Confirm password"),
        "pseudo" => t!("Pseudo (2 à 16 caractères)", "Username (2 to 16 characters)"),
        "btnLogin" => t!("Se connecter", "Log in"),
        "btnSignup" => t!("Créer le compte", "Create account"),
        "guest" => t!("Jouer en invité (sans XP)", "Play as guest (no XP)"),
        "localNote" => t!("Les comptes sont enregistrés sur cet ordinateur.", "Accounts are stored on this computer."),
        "noAccounts" => t!("Aucun compte sur cet ordinateur : crée le tien !", "No account on this computer yet: create yours!"),
        "navPlay" => t!("Jouer", "Play"),
        "navStats" => t!("Statistiques", "Statistics"),
        "navSettings" => t!("Paramètres", "Settings"),
        "navLogout" => t!("Déconnexion", "Log out"),
        "badge" => t!("NIV {n}", "LVL {n}"),
        "levelN" => t!("Niveau {n}", "Level {n}"),
        "guestName" => t!("Invité", "Guest"),
        "levelShort" => t!("niv. {n}", "lvl {n}"),
        "mRandomT" => t!("Adversaire aléatoire", "Random opponent"),
        "mRandomD" => t!("Affrontez un joueur en ligne au hasard (Bot après 45s).", "Face a random online player (bot after 45s)."),
        "mBotT" => t!("Partie contre un bot", "Play against a bot"),
        "mBotD" => t!("Entraînez-vous immédiatement contre l'IA tactique.", "Train right away against the tactical AI."),
        "mJoinT" => t!("Rejoindre une partie", "Join a game"),
        "mJoinD" => t!("Entrez un code de session unique pour rejoindre un ami.", "Enter a unique session code to join a friend."),
        "mHostT" => t!("Hôte", "Host"),
        "mHostD" => t!("Créez un salon privé et partagez votre code de session.", "Create a private room and share your session code."),
        "statsTitle" => t!("Statistiques de carrière", "Career statistics"),
        "combatTitle" => t!("Combat", "Combat"),
        "stTime" => t!("Temps joué", "Time played"),
        "stGames" => t!("Parties faites", "Games played"),
        "stWins" => t!("Victoires", "Wins"),
        "stLosses" => t!("Défaites", "Losses"),
        "stWinPct" => t!("% Victoire", "Win %"),
        "stLossPct" => t!("% Défaite", "Loss %"),
        "stWinRate" => t!("Taux de victoire", "Win rate"),
        "stLossRate" => t!("Taux de défaite", "Loss rate"),
        "stShots" => t!("Tirs", "Shots"),
        "stAcc" => t!("Précision", "Accuracy"),
        "stSunk" => t!("Bateaux coulés", "Ships sunk"),
        "stUpg" => t!("Améliorations utilisées", "Upgrades used"),
        "stStreak" => t!("Série en cours", "Current streak"),
        "stBest" => t!("Meilleure série", "Best streak"),
        "stFast" => t!("Victoire la plus rapide", "Fastest win"),
        "stInt" => t!("Parties interrompues", "Interrupted games"),
        "shotsN" => t!("{n} tirs", "{n} shots"),
        "guestStats" => t!("Crée un compte pour suivre tes statistiques.", "Create an account to track your statistics."),
        "setTitle" => t!("Paramètres", "Settings"),
        "sndT" => t!("Effets sonores", "Sound effects"),
        "sndD" => t!("Activer ou désactiver les bruitages navals", "Turn naval sound effects on or off"),
        "langT" => t!("Langue du jeu", "Game language"),
        "langD" => t!("Traduction intégrale de l'interface", "Full interface translation"),
        "photoT" => t!("Photo de profil", "Profile picture"),
        "photoD" => t!("Clique aussi sur l'avatar en haut à gauche pour la changer.", "You can also click the avatar at the top left to change it."),
        "photoChange" => t!("Changer la photo", "Change picture"),
        "photoRemove" => t!("Retirer", "Remove"),
        "editT" => t!("Modifier le pseudo et/ou le mot de passe", "Change username and/or password"),
        "newPseudo" => t!("Nouveau pseudo", "New username"),
        "newPass" => t!("Nouveau mot de passe (laisser vide pour conserver)", "New password (leave empty to keep)"),
        "save" => t!("Enregistrer les modifications", "Save changes"),
        "saved" => t!("Modifications enregistrées.", "Changes saved."),
        "nothingChanged" => t!("Aucune modification.", "Nothing to change."),
        "deleteT" => t!("Supprimer le compte", "Delete account"),
        "deleteD" => t!("Efface le compte, son XP et ses statistiques.", "Erases the account, its XP and statistics."),
        "deleteBtn" => t!("Supprimer", "Delete"),
        "deleteConfirm" => t!("Supprimer définitivement le compte « {name} », son XP et ses statistiques ?", "Permanently delete the account \"{name}\", its XP and statistics?"),
        "errPseudoLen" => t!("Le pseudo doit faire entre 2 et 16 caractères.", "Username must be 2 to 16 characters long."),
        "errPseudoTaken" => t!("Ce pseudo existe déjà sur cet ordinateur.", "This username already exists on this computer."),
        "errPassLen" => t!("Le mot de passe doit faire au moins 4 caractères.", "Password must be at least 4 characters long."),
        "errPassMatch" => t!("Les mots de passe ne correspondent pas.", "Passwords do not match."),
        "errPassWrong" => t!("Mot de passe incorrect.", "Wrong password."),
        "errPickAcc" => t!("Choisis un compte.", "Pick an account."),
        "errStorage" => t!("Impossible d'enregistrer le compte sur le disque.", "Cannot save the account to disk."),
        "errPhoto" => t!("Image illisible.", "Unreadable image."),
        "noPassHint" => t!("Ce compte n'a pas de mot de passe : ajoute-en un dans Paramètres.", "This account has no password: add one in Settings."),
        "btnJoin" => t!("Rejoindre", "Join"),
        "cancel" => t!("Annuler", "Cancel"),
        "connecting" => t!("Connexion…", "Connecting…"),
        "errCodeLen" => t!("Le code doit contenir 6 caractères.", "The code must be 6 characters long."),
        "errCodeChars" => t!("Code invalide : lettres A–Z (sauf I et O) et chiffres 2–9.", "Invalid code: letters A–Z (except I and O) and digits 2–9."),
        "errNoGame" => t!("Aucune partie avec ce code", "No game with this code"),
        "errFull" => t!("Cette partie est déjà complète.", "This game is already full."),
        "errJoinTimeout" => t!("Connexion impossible. Le réseau bloque peut-être la liaison directe.", "Cannot connect. The network may be blocking direct connections."),
        "errNet" => t!("Impossible de joindre le service de mise en relation. Vérifie ta connexion.", "Cannot reach the matchmaking service. Check your connection."),
        "errNetOther" => t!("Erreur réseau ({type}).", "Network error ({type})."),
        "lobbyHostT" => t!("Salon privé", "Private room"),
        "lobbyHostD" => t!("Donne ce code à ton adversaire :", "Give this code to your opponent:"),
        "copyCode" => t!("Copier le code", "Copy code"),
        "copyLink" => t!("Copier le lien", "Copy link"),
        "copied" => t!("Copié !", "Copied!"),
        "copyFail" => t!("Copie impossible : note le code à la main.", "Cannot copy: write the code down."),
        "creating" => t!("Création de la partie…", "Creating the game…"),
        "waitingOpp" => t!("En attente d'un adversaire…", "Waiting for an opponent…"),
        "reconnecting" => t!("Reconnexion au service de mise en relation…", "Reconnecting to the matchmaking service…"),
        "lobbyRandomT" => t!("Recherche d'un adversaire", "Looking for an opponent"),
        "lobbyRandomD" => t!("Un joueur en ligne au hasard. Sinon, le bot prendra sa place.", "A random online player. Otherwise, the bot takes over."),
        "searching" => t!("Recherche en cours…", "Searching…"),
        "countdown" => t!("Bot dans {s} s", "Bot in {s}s"),
        "noOneFound" => t!("Aucun joueur trouvé : place au bot !", "No player found: here comes the bot!"),
        "placeTitle" => t!("Place ta flotte", "Deploy your fleet"),
        "placeHelp" => t!(
            "Clique une case pour voir le bateau, clique-le à nouveau pour le poser. Fais glisser un bateau posé pour le déplacer. Raccourcis : R pivote, Entrée pose, Suppr retire.",
            "Click a square to preview the ship, click it again to place it. Drag a placed ship to move it. Shortcuts: R rotates, Enter places, Del removes."
        ),
        "rotH" => t!("Pivoter ↔", "Rotate ↔"),
        "rotV" => t!("Pivoter ↕", "Rotate ↕"),
        "random" => t!("Aléatoire", "Random"),
        "clear" => t!("Effacer", "Clear"),
        "confirm" => t!("Poser", "Place"),
        "remove" => t!("Retirer", "Remove"),
        "ready" => t!("Prêt", "Ready"),
        "pSelPlaced" => t!("{ship} sélectionné : fais-le glisser, pivote-le ou retire-le.", "{ship} selected: drag it, rotate it or remove it."),
        "pSelNew" => t!("{ship} : clique une case de départ.", "{ship}: click a starting square."),
        "pBad" => t!("Placement impossible : le bateau dépasse de la grille ou chevauche un autre bateau.", "Cannot place: the ship goes off the grid or overlaps another ship."),
        "pPlacedNext" => t!("{ship} posé. Au tour du {next}.", "{ship} placed. Next: {next}."),
        "pDone" => t!("Flotte complète ! Appuie sur Prêt.", "Fleet complete! Press Ready."),
        "pAllPlaced" => t!("Toute la flotte est posée. Clique un bateau pour le modifier.", "The whole fleet is placed. Click a ship to change it."),
        "pTapGhost" => t!("Clique le bateau vert (ou « Poser ») pour le valider.", "Click the green ship (or \"Place\") to confirm."),
        "pNoFit" => t!("Ici le bateau ne rentre pas : pivote-le ou choisis une autre case.", "The ship does not fit here: rotate it or pick another square."),
        "pBlocked" => t!("Position bloquée par un autre bateau.", "Blocked by another ship."),
        "pRemoved" => t!("{ship} retiré.", "{ship} removed."),
        "pCleared" => t!("Grille vidée.", "Grid cleared."),
        "pRandom" => t!("Flotte placée au hasard. Tu peux encore déplacer les bateaux.", "Fleet placed at random. You can still move the ships."),
        "pRelease" => t!("Relâche pour poser.", "Release to place."),
        "pNotHere" => t!("Impossible ici.", "Not here."),
        "pRestored" => t!("Placement impossible : {ship} remis à sa place.", "Cannot place: {ship} put back."),
        "waitOppReady" => t!("Flotte prête. En attente de l'adversaire…", "Fleet ready. Waiting for the opponent…"),
        "oppReady" => t!("L'adversaire est prêt.", "The opponent is ready."),
        "oppAway" => t!("L'adversaire a quitté l'écran du jeu… (2 min max)", "The opponent left the game screen… (2 min max)"),
        "ship0" => t!("Porte-avions", "Carrier"),
        "ship1" => t!("Croiseur", "Battleship"),
        "ship2" => t!("Contre-torpilleur", "Destroyer"),
        "ship3" => t!("Sous-marin", "Submarine"),
        "ship4" => t!("Torpilleur", "Patrol boat"),
        "stMyTurn" => t!("À toi de jouer", "Your turn"),
        "stOppTurn" => t!("Tour de l'adversaire", "Opponent's turn"),
        "stFiring" => t!("Tir en cours", "Firing…"),
        "stOver" => t!("Partie terminée", "Game over"),
        "stLost" => t!("Connexion perdue", "Connection lost"),
        "enemyGrid" => t!("Grille de {name}", "{name}'s grid"),
        "myFleet" => t!("Ta flotte", "Your fleet"),
        "modeShot" => t!("Tir", "Shot"),
        "modeRadar" => t!("Radar", "Radar"),
        "modeCross" => t!("Croix", "Cross"),
        "modeShield" => t!("Bouclier", "Shield"),
        "hintShot" => t!("Clique une case de la grille adverse pour tirer.", "Click a square on the enemy grid to fire."),
        "hintRadar" => t!("Radar : clique le coin haut-gauche de la zone 2 × 2 à scanner.", "Radar: click the top-left square of the 2 × 2 area to scan."),
        "hintCross" => t!("Frappe en croix : clique la case centrale.", "Cross strike: click the center square."),
        "hintShield" => t!("Bouclier : clique un de tes bateaux dans ta flotte.", "Shield: click one of your ships in your fleet."),
        "fWaitTurn" => t!("Attends ton tour.", "Wait for your turn."),
        "fFiring" => t!("Tir en cours…", "Firing…"),
        "fShieldHere" => t!("Pour le bouclier, clique un bateau de ta flotte.", "For the shield, click a ship in your fleet."),
        "fAlready" => t!("Case déjà visée.", "Square already targeted."),
        "fNoUnknown" => t!("Aucune case inconnue dans cette zone.", "No unknown square in this area."),
        "fAllKnown" => t!("Toutes ces cases sont déjà connues.", "All these squares are already known."),
        "fTouchShip" => t!("Clique une case occupée par un de tes bateaux.", "Click a square holding one of your ships."),
        "fShipSunk" => t!("Ce bateau est déjà coulé.", "This ship is already sunk."),
        "fHasShield" => t!("Ce bateau a déjà reçu un bouclier.", "This ship already got a shield."),
        "shieldPlaced" => t!("Bouclier posé sur ton {ship}.", "Shield placed on your {ship}."),
        "oppShield" => t!("{name} — bouclier posé sur un de ses bateaux.", "{name} — shielded one of their ships."),
        "startHost" => t!("La partie commence : tu joues en premier.", "The game starts: you fire first."),
        "startGuest" => t!("La partie commence : l'adversaire joue en premier.", "The game starts: your opponent fires first."),
        "oppPrefix" => t!("{name} — ", "{name} — "),
        "and" => t!(" et ", " and "),
        "upRadarN" => t!("Radar", "Radar"),
        "upRadarD" => t!("Scanne une zone 2 × 2 à partir de la case visée. Coûte le tour.", "Scans a 2 × 2 area from the targeted square. Costs your turn."),
        "upCrossN" => t!("Frappe en croix", "Cross strike"),
        "upCrossD" => t!("Tire sur la case visée et ses 4 voisines. Coûte le tour.", "Fires at the targeted square and its 4 neighbours. Costs your turn."),
        "upShieldN" => t!("Bouclier", "Shield"),
        "upShieldD" => t!("Le premier coup sur un de tes bateaux est annulé. Coûte le tour, sans tir.", "The first hit on one of your ships is cancelled. Costs your turn, no shot."),
        "choiceTitle" => t!("Bateau ennemi coulé !", "Enemy ship sunk!"),
        "choiceSub" => t!("Choisis une amélioration :", "Pick an upgrade:"),
        "choiceSubN" => t!("Choisis une amélioration ({n} à choisir) :", "Pick an upgrade ({n} to pick):"),
        "win" => t!("Victoire !", "Victory!"),
        "lose" => t!("Défaite…", "Defeat…"),
        "winText" => t!("Tu as coulé toute la flotte adverse.", "You sank the whole enemy fleet."),
        "loseText" => t!("Toute ta flotte a été coulée.", "Your whole fleet was sunk."),
        "lostTitle" => t!("Connexion perdue", "Connection lost"),
        "lostText" => t!("La liaison avec l'adversaire est coupée. La partie est arrêtée.", "The link with your opponent is broken. The game is stopped."),
        "newGame" => t!("Nouvelle partie", "New game"),
        "seeGrids" => t!("Voir les grilles", "View grids"),
        "xpGuest" => t!("Joue avec un compte pour gagner de l'XP.", "Play with an account to earn XP."),
        "levelUp" => t!("Niveau {n} : {rank} !", "Level {n}: {rank}!"),
        "botXp" => t!("XP réduite de moitié contre le bot.", "XP halved against the bot."),
        "quit" => t!("Quitter", "Quit"),
        "quitConfirm" => t!("Quitter la partie ? Elle sera comptée comme interrompue.", "Quit the game? It will count as interrupted."),
        "botName" => t!("Bot tactique", "Tactical bot"),
        "vs" => t!("contre", "vs"),
        "welcome" => t!("Bienvenue, {name} !", "Welcome, {name}!"),
        "yes" => t!("Oui", "Yes"),
        "no" => t!("Non", "No"),
        "back" => t!("Retour", "Back"),
        "photoPick" => t!("Choisir une photo", "Pick a picture"),
        "images" => t!("Images", "Images"),
        "updChecking" => t!("Recherche de mises à jour…", "Checking for updates…"),
        "updTitle" => t!("Mise à jour obligatoire", "Mandatory update"),
        "updText" => t!("La version {to} est disponible (tu as la {from}). Elle s'installe automatiquement.", "Version {to} is available (you have {from}). It is being installed automatically."),
        "updDownloading" => t!("Téléchargement… {pct}", "Downloading… {pct}"),
        "updRestarting" => t!("Installation terminée : redémarrage…", "Update installed: restarting…"),
        "updFailed" => t!("La mise à jour a échoué. Télécharge la nouvelle version sur la page des versions, puis remplace ce fichier.", "The update failed. Download the new version from the releases page, then replace this file."),
        "updOpen" => t!("Ouvrir la page des versions", "Open the releases page"),
        "updQuit" => t!("Quitter", "Quit"),
        "rulesUnsupported" => t!(
            "L'hôte utilise des règles personnalisées (grille ou flotte) que cette version Windows ne gère pas encore.",
            "The host uses custom rules (grid or fleet) that this Windows version does not support yet."
        ),
        _ => "?",
    }
}

pub fn ranks(lang: Lang) -> [&'static str; 11] {
    crate::i18n_data::RANKS[lang.idx()]
}

/// Noms du bateau n° `i` de la flotte `lens` (deux bateaux de même longueur : deux noms différents).
pub fn ship_name(lang: Lang, lens: &[usize], i: usize) -> String {
    let len = lens[i];
    let list: &[&str] = crate::i18n_data::SHIPS[lang.idx()].get(len.wrapping_sub(1)).copied().unwrap_or(&[]);
    let same = lens.iter().filter(|&&l| l == len).count();
    let occ = lens[..i].iter().filter(|&&l| l == len).count();
    let first = list.first().copied().unwrap_or("?");
    if same == 1 {
        return first.to_string();
    }
    list.get(occ).map_or_else(|| format!("{first} {}", occ + 1), |s| s.to_string())
}

pub fn theme_name(lang: Lang, k: usize) -> &'static str {
    crate::i18n_data::THEMES[lang.idx()][k]
}

/// Pluriel : en français 0 et 1 sont au singulier, dans les autres langues seul 1 l'est.
fn pln(lang: Lang, n: usize, one: &str, many: &str) -> String {
    let sing = if lang == Lang::Fr { n <= 1 } else { n == 1 };
    format!("{n} {}", if sing { one } else { many })
}

pub fn sum_radar(lang: Lang, cell: &str, n: usize, total: usize) -> String {
    let one = total == 1;
    match lang {
        Lang::Fr => format!("Radar en {cell} : {} sur {total}.", pl(n, "case occupée", "cases occupées")),
        Lang::En => format!("Radar at {cell}: {n} of {total} square{} occupied.", if total > 1 { "s" } else { "" }),
        Lang::Pt => format!("Radar em {cell}: {n} de {total} {}.", if one { "casa ocupada" } else { "casas ocupadas" }),
        Lang::Es => format!("Radar en {cell}: {n} de {total} {}.", if one { "casilla ocupada" } else { "casillas ocupadas" }),
        Lang::De => format!("Radar auf {cell}: {n} von {total} {} besetzt.", if one { "Feld" } else { "Feldern" }),
        Lang::It => format!("Radar su {cell}: {n} di {total} {}.", if one { "casella occupata" } else { "caselle occupate" }),
    }
}

pub fn sum_shot(lang: Lang, cell: &str, v: &str, ship: &str) -> String {
    let (head, miss, hit, blocked, sunk) = match lang {
        Lang::Fr => ("Tir en", "raté.", "touché !", "bloqué par un bouclier.", format!("coulé ! ({ship})")),
        Lang::En => ("Shot at", "miss.", "hit!", "blocked by a shield.", format!("sunk! ({ship})")),
        Lang::Pt => ("Tiro em", "água.", "acertou!", "bloqueado por um escudo.", format!("afundado! ({ship})")),
        Lang::Es => ("Disparo en", "agua.", "¡tocado!", "bloqueado por un escudo.", format!("¡hundido! ({ship})")),
        Lang::De => ("Schuss auf", "Wasser.", "Treffer!", "vom Schild abgewehrt.", format!("versenkt! ({ship})")),
        Lang::It => ("Colpo su", "acqua.", "colpito!", "bloccato da uno scudo.", format!("affondata! ({ship})")),
    };
    let r = match v {
        "miss" => miss.to_string(),
        "hit" => hit.to_string(),
        "blocked" => blocked.to_string(),
        "sunk" => sunk,
        _ => String::new(),
    };
    let colon = if lang == Lang::Fr { " :" } else { ":" };
    format!("{head} {cell}{colon} {r}")
}

pub fn sum_cross(lang: Lang, cell: &str, hits: usize, miss: usize, blk: usize, sunk: &str, sunk_n: usize) -> String {
    match lang {
        Lang::Fr => format!(
            "Frappe en croix sur {cell} : {}, {}{}.{}",
            pl(hits, "touché", "touchés"),
            pl(miss, "raté", "ratés"),
            if blk > 0 { format!(", {}", pl(blk, "bloqué", "bloqués")) } else { String::new() },
            if sunk_n > 0 { format!(" {sunk} coulé{} !", if sunk_n > 1 { "s" } else { "" }) } else { String::new() }
        ),
        Lang::En => format!(
            "Cross strike at {cell}: {}, {}{}.{}",
            pln(lang, hits, "hit", "hits"),
            pln(lang, miss, "miss", "misses"),
            if blk > 0 { format!(", {blk} blocked") } else { String::new() },
            if sunk_n > 0 { format!(" {sunk} sunk!") } else { String::new() }
        ),
        Lang::Pt => format!(
            "Ataque em cruz em {cell}: {}, {}{}.{}",
            pln(lang, hits, "acerto", "acertos"),
            pln(lang, miss, "falha", "falhas"),
            if blk > 0 { format!(", {blk} bloqueado(s)") } else { String::new() },
            if sunk_n > 0 { format!(" {sunk} afundado(s)!") } else { String::new() }
        ),
        Lang::Es => format!(
            "Ataque en cruz en {cell}: {}, {}{}.{}",
            pln(lang, hits, "tocado", "tocados"),
            pln(lang, miss, "agua", "aguas"),
            if blk > 0 { format!(", {blk} bloqueado(s)") } else { String::new() },
            if sunk_n > 0 { format!(" ¡{sunk} hundido(s)!") } else { String::new() }
        ),
        Lang::De => format!(
            "Kreuzschlag auf {cell}: {}, {miss} × Wasser{}.{}",
            pln(lang, hits, "Treffer", "Treffer"),
            if blk > 0 { format!(", {blk} abgewehrt") } else { String::new() },
            if sunk_n > 0 { format!(" {sunk} versenkt!") } else { String::new() }
        ),
        Lang::It => format!(
            "Attacco a croce su {cell}: {}, {}{}.{}",
            pln(lang, hits, "colpo a segno", "colpi a segno"),
            pln(lang, miss, "a vuoto", "a vuoto"),
            if blk > 0 { format!(", {blk} bloccato/i") } else { String::new() },
            if sunk_n > 0 { format!(" {sunk} affondata/e!") } else { String::new() }
        ),
    }
}

pub fn end_stats(lang: Lang, shots: usize, acc: u32) -> String {
    match lang {
        Lang::Fr => format!("{}, {acc} % de précision.", pl(shots, "tir", "tirs")),
        Lang::En => format!("{}, {acc}% accuracy.", pln(lang, shots, "shot", "shots")),
        Lang::Pt => format!("{}, {acc} % de precisão.", pln(lang, shots, "tiro", "tiros")),
        Lang::Es => format!("{}, {acc} % de precisión.", pln(lang, shots, "disparo", "disparos")),
        Lang::De => format!("{}, {acc} % Trefferquote.", pln(lang, shots, "Schuss", "Schüsse")),
        Lang::It => format!("{}, {acc} % di precisione.", pln(lang, shots, "colpo", "colpi")),
    }
}

/// Mots des résumés de règles, par langue : (bateau, bateaux, case, cases, « Règles », « grille », « sans améliorations »).
fn rules_words(l: Lang) -> (&'static str, &'static str, &'static str, &'static str, &'static str, &'static str, &'static str, &'static str) {
    match l {
        Lang::Fr => ("bateau", "bateaux", "case", "cases", "Règles : grille", "de la grille", ", sans améliorations", ""),
        Lang::En => ("ship", "ships", "square", "squares", "Rules:", "of the grid", ", no upgrades", "grid"),
        Lang::Pt => ("navio", "navios", "casa", "casas", "Regras: grelha", "da grelha", ", sem melhorias", ""),
        Lang::Es => ("barco", "barcos", "casilla", "casillas", "Reglas: cuadrícula", "de la cuadrícula", ", sin mejoras", ""),
        Lang::De => ("Schiff", "Schiffe", "Feld", "Felder", "Regeln: Feld", "des Spielfelds", ", ohne Extras", ""),
        Lang::It => ("nave", "navi", "casella", "caselle", "Regole: griglia", "della griglia", ", senza potenziamenti", ""),
    }
}

/// Total de la flotte : « 5 bateaux · 17 cases (17 % de la grille) ».
pub fn fleet_total(lang: Lang, ships: usize, cells: usize, pct: usize) -> String {
    let w = rules_words(lang);
    let sep = if lang == Lang::En { "" } else { " " };
    format!("{} · {} ({pct}{sep}% {})", pln(lang, ships, w.0, w.1), pln(lang, cells, w.2, w.3), w.5).replace(" % of", "% of")
}

/// Résumé des règles affiché au placement.
pub fn rules_info(lang: Lang, size: usize, ships: usize, upgrades: bool) -> String {
    let w = rules_words(lang);
    let up = if upgrades { "" } else { w.6 };
    match lang {
        Lang::En => format!("Rules: {size} × {size} grid, {}{up}.", pln(lang, ships, w.0, w.1)),
        _ => format!("{} {size} × {size}, {}{up}.", w.4, pln(lang, ships, w.0, w.1)),
    }
}

/// Résumé des règles personnalisées (accueil).
pub fn rules_custom(lang: Lang, size: usize, ships: usize) -> String {
    let w = rules_words(lang);
    let head = match lang {
        Lang::Fr => "Règles :",
        Lang::En => "Rules:",
        Lang::Pt => "Regras:",
        Lang::Es => "Reglas:",
        Lang::De => "Regeln:",
        Lang::It => "Regole:",
    };
    format!("{head} {size} × {size}, {}", pln(lang, ships, w.0, w.1))
}

/// Textes propres à l'application Windows (mise à jour, sélection de photo…) pour les langues
/// absentes du tableau principal ; le français et l'anglais sont dans `tr`.
fn own(lang: Lang, key: &str) -> Option<&'static str> {
    let i = match lang {
        Lang::Pt => 0,
        Lang::Es => 1,
        Lang::De => 2,
        Lang::It => 3,
        _ => return None,
    };
    let v: [&'static str; 4] = match key {
        "yes" => ["Sim", "Sí", "Ja", "Sì"],
        "no" => ["Não", "No", "Nein", "No"],
        "back" => ["Voltar", "Volver", "Zurück", "Indietro"],
        "photoPick" => ["Escolher uma foto", "Elegir una foto", "Foto auswählen", "Scegli una foto"],
        "images" => ["Imagens", "Imágenes", "Bilder", "Immagini"],
        "updChecking" => ["A procurar atualizações…", "Buscando actualizaciones…", "Suche nach Updates…", "Ricerca di aggiornamenti…"],
        "updTitle" => ["Atualização obrigatória", "Actualización obligatoria", "Pflicht-Update", "Aggiornamento obbligatorio"],
        "updText" => [
            "A versão {to} está disponível (tens a {from}). Está a ser instalada automaticamente.",
            "La versión {to} está disponible (tienes la {from}). Se está instalando automáticamente.",
            "Version {to} ist verfügbar (du hast {from}). Sie wird automatisch installiert.",
            "La versione {to} è disponibile (hai la {from}). Viene installata automaticamente.",
        ],
        "updDownloading" => ["A transferir… {pct}", "Descargando… {pct}", "Wird heruntergeladen… {pct}", "Download… {pct}"],
        "updRestarting" => [
            "Instalação concluída: a reiniciar…",
            "Instalación terminada: reiniciando…",
            "Update installiert: Neustart…",
            "Installazione completata: riavvio…",
        ],
        "updFailed" => [
            "A atualização falhou. Transfere a nova versão na página de versões e substitui este ficheiro.",
            "La actualización ha fallado. Descarga la nueva versión en la página de versiones y reemplaza este archivo.",
            "Das Update ist fehlgeschlagen. Lade die neue Version von der Release-Seite herunter und ersetze diese Datei.",
            "L'aggiornamento non è riuscito. Scarica la nuova versione dalla pagina delle versioni e sostituisci questo file.",
        ],
        "updOpen" => ["Abrir a página de versões", "Abrir la página de versiones", "Release-Seite öffnen", "Apri la pagina delle versioni"],
        "updQuit" => ["Sair", "Salir", "Beenden", "Esci"],
        "localNote" => [
            "As contas são guardadas neste computador.",
            "Las cuentas se guardan en este ordenador.",
            "Konten werden auf diesem Computer gespeichert.",
            "Gli account sono salvati su questo computer.",
        ],
        "noAccounts" => [
            "Nenhuma conta neste computador: cria a tua!",
            "No hay cuentas en este ordenador: ¡crea la tuya!",
            "Noch kein Konto auf diesem Computer: Erstelle deins!",
            "Nessun account su questo computer: creane uno!",
        ],
        "photoT" => ["Foto de perfil", "Foto de perfil", "Profilbild", "Foto del profilo"],
        "photoD" => [
            "Clica também no avatar no canto superior esquerdo para a alterar.",
            "Haz clic también en el avatar de arriba a la izquierda para cambiarla.",
            "Klicke auch auf den Avatar oben links, um es zu ändern.",
            "Fai clic anche sull'avatar in alto a sinistra per cambiarla.",
        ],
        "photoChange" => ["Mudar a foto", "Cambiar la foto", "Foto ändern", "Cambia foto"],
        "editT" => [
            "Alterar o nome e/ou a palavra-passe",
            "Cambiar el nombre y/o la contraseña",
            "Namen und/oder Passwort ändern",
            "Cambia il nome e/o la password",
        ],
        "errPseudoTaken" => [
            "Este nome já existe neste computador.",
            "Este nombre ya existe en este ordenador.",
            "Dieser Name existiert auf diesem Computer bereits.",
            "Questo nome esiste già su questo computer.",
        ],
        "errStorage" => [
            "Não foi possível guardar a conta no disco.",
            "No se puede guardar la cuenta en el disco.",
            "Das Konto konnte nicht auf der Festplatte gespeichert werden.",
            "Impossibile salvare l'account sul disco.",
        ],
        "reconnecting" => [
            "A religar ao serviço de ligação…",
            "Reconectando con el servicio de emparejamiento…",
            "Verbindung zum Vermittlungsdienst wird wiederhergestellt…",
            "Riconnessione al servizio di abbinamento…",
        ],
        "rulesUnsupported" => [
            "O anfitrião usa regras personalizadas (grelha ou frota) que esta versão Windows ainda não suporta.",
            "El anfitrión usa reglas personalizadas (cuadrícula o flota) que esta versión de Windows aún no admite.",
            "Der Gastgeber nutzt eigene Regeln (Raster oder Flotte), die diese Windows-Version noch nicht unterstützt.",
            "L'host usa regole personalizzate (griglia o flotta) che questa versione per Windows non supporta ancora.",
        ],
        _ => return None,
    };
    Some(v[i])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n_data::*;

    #[test]
    fn tables_are_sorted_for_binary_search() {
        for t in [FR, EN, PT, ES, DE, IT] {
            assert!(t.windows(2).all(|w| w[0].0 < w[1].0), "table non triée");
        }
    }

    #[test]
    fn every_key_has_a_text_in_every_language() {
        // les clés du tableau complet (italien) doivent exister, et être non vides, dans les six langues
        for (k, _) in IT {
            for l in Lang::ALL {
                let s = tr(l, k);
                assert!(s != "?" && !s.is_empty(), "clé {k} sans texte en {l:?}");
            }
        }
        for k in ["yes", "no", "back", "photoPick", "images", "updTitle", "updText", "updQuit", "rulesUnsupported", "localNote", "photoChange"] {
            for l in Lang::ALL {
                assert!(tr(l, k) != "?", "clé {k} sans texte en {l:?}");
            }
        }
    }

    /// Toute clé de texte employée dans le code (`t("clé")`, `tf("clé", …)`) doit avoir un texte dans chaque langue.
    #[test]
    fn keys_used_in_code_exist() {
        let mut checked = 0;
        for src in [include_str!("app.rs"), include_str!("ui.rs"), include_str!("update.rs")] {
            for pat in [".t(\"", ".tf(\"", "tr(l, \"", "tr(self.lang(), \"", "tr(self.settings.lang, \""] {
                for part in src.split(pat).skip(1) {
                    let Some(key) = part.split('"').next() else { continue };
                    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                        continue;
                    }
                    for l in Lang::ALL {
                        assert!(tr(l, key) != "?", "clé « {key} » sans texte en {l:?}");
                    }
                    checked += 1;
                }
            }
        }
        assert!(checked > 100, "seulement {checked} clés trouvées");
    }

    #[test]
    fn rules_texts() {
        assert_eq!(fleet_total(Lang::Fr, 5, 17, 17), "5 bateaux · 17 cases (17 % de la grille)");
        assert_eq!(fleet_total(Lang::En, 1, 1, 1), "1 ship · 1 square (1% of the grid)");
        assert_eq!(rules_info(Lang::Fr, 10, 5, false), "Règles : grille 10 × 10, 5 bateaux, sans améliorations.");
        assert_eq!(rules_info(Lang::En, 8, 3, true), "Rules: 8 × 8 grid, 3 ships.");
        for l in Lang::ALL {
            assert!(rules_info(l, 10, 5, true).contains("10 × 10"));
            assert!(rules_custom(l, 12, 4).contains("12 × 12"));
            assert!(fleet_total(l, 5, 17, 17).contains("17"));
        }
    }

    #[test]
    fn ship_names_follow_fleet() {
        let lens = [5, 4, 3, 3, 2];
        assert_eq!(ship_name(Lang::En, &lens, 0), "Carrier");
        assert_eq!(ship_name(Lang::En, &lens, 2), "Destroyer");
        assert_eq!(ship_name(Lang::En, &lens, 3), "Submarine");
        assert_eq!(ship_name(Lang::Fr, &lens, 4), "Torpilleur");
    }

    #[test]
    fn summaries_in_all_languages() {
        for l in Lang::ALL {
            assert!(sum_radar(l, "C5", 2, 4).contains("C5"));
            assert!(sum_shot(l, "C5", "sunk", "X").contains('X'));
            assert!(sum_cross(l, "C5", 2, 3, 1, "X", 1).contains("C5"));
            assert!(end_stats(l, 12, 50).contains("12"));
        }
    }
}

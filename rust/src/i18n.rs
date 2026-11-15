//! Textes de l'interface (français / anglais).

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    Fr,
    En,
}

impl Lang {
    pub fn detect() -> Lang {
        // Langue de Windows : français si l'interface utilisateur est en français.
        let l = std::env::var("LANG").unwrap_or_default().to_lowercase();
        if l.starts_with("en") {
            return Lang::En;
        }
        #[cfg(windows)]
        {
            extern "system" {
                fn GetUserDefaultUILanguage() -> u16;
            }
            let id = unsafe { GetUserDefaultUILanguage() };
            if id & 0x3ff != 0x0c {
                return Lang::En;
            }
        }
        Lang::Fr
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
        _ => "?",
    }
}

pub fn ranks(lang: Lang) -> [&'static str; 11] {
    match lang {
        Lang::Fr => [
            "Mousse", "Matelot", "Quartier-maître", "Second-maître", "Maître", "Enseigne", "Lieutenant",
            "Capitaine de corvette", "Capitaine de frégate", "Capitaine de vaisseau", "Amiral",
        ],
        Lang::En => [
            "Cabin boy", "Sailor", "Leading seaman", "Petty officer", "Chief petty officer", "Ensign", "Lieutenant",
            "Lieutenant commander", "Commander", "Captain", "Admiral",
        ],
    }
}

pub fn sum_radar(lang: Lang, cell: &str, n: usize, total: usize) -> String {
    match lang {
        Lang::Fr => format!("Radar en {cell} : {} sur {total}.", pl(n, "case occupée", "cases occupées")),
        Lang::En => format!("Radar at {cell}: {n} of {total} square{} occupied.", if total > 1 { "s" } else { "" }),
    }
}

pub fn sum_shot(lang: Lang, cell: &str, v: &str, ship: &str) -> String {
    match lang {
        Lang::Fr => {
            let r = match v {
                "miss" => "raté.".to_string(),
                "hit" => "touché !".to_string(),
                "blocked" => "bloqué par un bouclier.".to_string(),
                "sunk" => format!("coulé ! ({ship})"),
                _ => String::new(),
            };
            format!("Tir en {cell} : {r}")
        }
        Lang::En => {
            let r = match v {
                "miss" => "miss.".to_string(),
                "hit" => "hit!".to_string(),
                "blocked" => "blocked by a shield.".to_string(),
                "sunk" => format!("sunk! ({ship})"),
                _ => String::new(),
            };
            format!("Shot at {cell}: {r}")
        }
    }
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
            pl(hits, "hit", "hits"),
            pl(miss, "miss", "misses"),
            if blk > 0 { format!(", {blk} blocked") } else { String::new() },
            if sunk_n > 0 { format!(" {sunk} sunk!") } else { String::new() }
        ),
    }
}

pub fn end_stats(lang: Lang, shots: usize, acc: u32) -> String {
    match lang {
        Lang::Fr => format!("{}, {acc} % de précision.", pl(shots, "tir", "tirs")),
        Lang::En => format!("{}, {acc}% accuracy.", pl(shots, "shot", "shots")),
    }
}

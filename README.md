# ⚓ C02 Naval

Bataille navale à deux joueurs, dans le navigateur, sur PC ou téléphone. Sans installation, sans serveur de jeu.

## Jouer

1. Un joueur clique sur **Créer une partie** et obtient un code de 6 caractères.
2. L'autre joueur saisit ce code et clique sur **Rejoindre**.
3. Chacun place sa flotte, puis appuie sur **Prêt**. L'hôte tire en premier.

Chaque bateau ennemi coulé donne une amélioration au choix : **Radar** (zone 2 × 2), **Frappe en croix** (5 cases) ou **Bouclier** (annule le premier coup sur un de tes bateaux).

## Comptes

Crée un compte (pseudo + avatar) pour gagner de l'XP, monter de niveau (de Mousse à Amiral) et suivre tes statistiques : victoires, précision, séries, bateaux coulés…
Les comptes sont enregistrés **dans le navigateur de l'appareil** : ils ne se synchronisent pas entre PC et téléphone, et effacer les données du site efface le compte.

## Technique

- Un seul fichier : `index.html` (HTML, CSS et JavaScript).
- Connexion pair-à-pair (WebRTC) avec [PeerJS](https://peerjs.com) 1.5.4. Le service public de PeerJS sert uniquement à mettre les deux navigateurs en relation.
- Sur les réseaux très fermés, la connexion directe peut échouer : un relais TURN personnel peut être ajouté dans la constante `MY_TURN` de `index.html`.

## Tester en local

```bash
npm install
npm start
```

Puis ouvrir `http://localhost:8080` (sur un téléphone du même Wi-Fi : `http://<IP-du-PC>:8080`).

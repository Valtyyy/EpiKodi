# ADR 0002 : Composition vidéo, webview transparent sur `GtkGLArea` libmpv (plan A) avec repli plan B

- **Statut :** Accepté, sous conditions de réévaluation (voir « Risques et suivi »)
- **Date :** 2026-11-12
- **Références :** `ARCHITECTURE.md` §1 (C1), §5.1, §5.2 (risque n°1), §16 ; issues #11 (cette ADR), #10 (spike), #57 (7.2 surface vidéo), #64 (7.9 contrôles de lecture) ; [rapport du spike 2.1](../spikes/2.1-composition-video.md)

## Contexte

C1 impose que toute lecture passe par `libmpv`. Or mpv ne peut pas dessiner dans un webview : les deux doivent être composés nativement (§5.2). La session de développement est sous Wayland, ce qui exclut les solutions propres à X11.

Le §5.2 posait une hypothèse non validée : *« aucune source confirmant un WebKitGTK transparent par-dessus une `GtkGLArea` »*. Le spike 2.1 (issue #10, code dans `spikes/video-overlay/`) l'a testée avec Tauri 2 (`wry` 0.57.1, `webkit2gtk` 2.0.2, `libmpv2` 7.0.1) sur une seule machine : Ubuntu 24.04, GNOME 46 Wayland et XWayland, AMD Radeon 680M, Mesa 25.2.8, libmpv 0.37.0, vidéo de test 1080p30 H.264. Le rapport est la source de tous les chiffres ci-dessous ; il précise qu'aucun n'est estimé.

## Décision

**Plan A est retenu sur AMD (Wayland et XWayland).** La racine de composition (`src-tauri/src/bootstrap.rs`, voir ADR-0003) retire le `WebKitWebView` du `default_vbox()` de la fenêtre et construit `GtkOverlay { GtkGLArea (rendu mpv par l'API render de libmpv), WebView transparent }`. Le webview est retrouvé parmi les enfants du vbox via `gtk_window()` et `default_vbox()`, sans `with_webview` ni contournement de Tauri. L'overlay ne laisse pas passer les événements (`set_overlay_pass_through(false)`) : le webview reçoit clavier et souris.

Règles d'implémentation validées par le spike (prolongent le §5.1) :

1. `LC_NUMERIC=C` est rétabli après l'initialisation de GTK et avant `mpv_create`.
2. Configuration fermée de mpv (`config=no`, `load-scripts=no`, `ytdl=no`, `input-default-bindings=no`, `input-vo-keyboard=no`, `osc=no`) et `vo=libmpv`.
3. Le contexte de rendu est créé dans le handler `realize` de la `GtkGLArea`, contexte GL courant, **avant** `loadfile`. Si la surface n'est pas réalisée, le fichier est mis en attente puis chargé par `realize`.
4. Le callback de mise à jour de mpv ne fait que poser un drapeau atomique et notifier le fil GTK, qui appelle `queue_render`. Aucun appel à l'API cliente n'y est fait.
5. Le rendu cible le FBO lié de la `GLArea` (`GL_FRAMEBUFFER_BINDING` interrogé à chaque rendu). `get_proc_address` passe par `eglGetProcAddress` (Wayland) ou `glXGetProcAddressARB` (X11), avec repli `dlsym`. Les pointeurs `wl_display` / `Display*` sont transmis à mpv (nécessaire à l'interop VAAPI zéro-copie).
6. `hwdec=auto-safe`.
7. Le fond du webview est transparent (`set_background_color` RGBA 0,0,0,0 et fenêtre `.transparent(true)`).

Ces règles ne sont pas toutes contrôlables par le compilateur : les règles 1, 3 et 4 le sont par revue et par le test de lecture de l'EPIC 07 ; la règle 7 l'est par revue du CSS de `ui/tv`.

**Repli : plan B.** `GtkStack { "ui" = webview, "video" = GLArea }`, avec le même code de rendu. Seuls le conteneur et le relais clavier changent : les touches de la fenêtre (`key-press-event`) sont relayées à mpv (Espace, flèches, `f`, Échap) et l'information passe par l'OSD (`show-text`, texte formaté côté Rust, car mpv ne développe pas les `${propriété}` via `libmpv2`). Le choix du plan est fait au démarrage, pas à la compilation, pour que la bascule soit peu coûteuse. *Proposition, à valider dans l'issue #57 :* variable d'environnement `OPEN_MOSAIK_VIDEO_PLAN=a|b`, doublée d'un réglage persistant dans `SettingsService`, la variable l'emportant.

## Alternatives écartées

- **`--wid` ou fenêtres empilées (X11 uniquement).** Écarté : propre à X11, alors que la session de développement est sous Wayland (§5.2).
- **`tauri-plugin-libmpv`.** Écarté : son README indique pour Linux « Window embedding is not working » (§5.2).
- **Balise HTML `<video>`.** Écarté : viole C1, qui exclut explicitement toute balise `<video>` ou `<audio>` (§1) et perd les pistes, les sous-titres SRT et la reprise gérés par `PlayerEngine`.
- **Plan B comme solution principale.** Écarté : il fonctionne et coûte moins de CPU (16,6 à 17,3 % contre 24,9 à 26,1 % en Wayland), mais il est moins riche et moins accessible. Plan A fonctionne et répond au besoin produit (contrôles HTML avec focus pendant la lecture). Plan B reste le repli.

## Conséquences

### Positives

- Hypothèse du §5.2 confirmée sur AMD, sans écart par rapport à Tauri : aucune erreur GL (`glGetError` après chaque rendu), 20 bascules plein écran (10 allers-retours) sans plantage, 5 touches sur 5 reçues par la page avec le focus resté dans le `WebKitWebView`, 0 perte décodeur.
- Les contrôles de lecture sont du HTML/React dans `ui/tv` (issue #64) : accessibles (annonces, lecteurs d'écran), thémés avec le reste de l'application, navigables par la navigation spatiale ; le focus reste dans le webview, sans relais de touches.
- Plan B existe déjà dans le spike et réutilise le même rendu : le repli est peu coûteux.

### Négatives et compromis

- **Coût CPU de l'overlay :** environ +9 à +10 points d'un cœur par rapport à la référence sans webview. Pire cas : le webview du test est redessiné à 20 Hz.
- **Images perdues :** la cible « < 1 % » est tenue en régime établi (0 perte en t=22..30 s sur tous les runs) mais pas toujours sur la fenêtre totale de 30 s. Les pertes se concentrent au démarrage et pendant les bascules plein écran.
- Contrainte sur `ui/tv` : pendant la lecture, le webview doit rester transparent. Aucun fond opaque (`html`, `body`, racine React) sur l'écran de lecture ; seuls les contrôles peignent.
- Le webview doit rester actif et affiché pendant la lecture. WebKit bride un webview caché (environ 3 ticks/s au lieu de 20 en plan B) : sans conséquence en plan A, à retenir pour le plan B si le webview devait pousser un état.
- Avec `WEBKIT_DISABLE_DMABUF_RENDERER=1` : environ +5 points de CPU sur un run. À n'activer qu'en cas d'écran noir (typiquement NVIDIA).
- Dépendance à `glib::MainContext::channel`, déprécié dans glib 0.18 mais seul idiome pour la paire gtk/glib de Tauri 2.12 (`#![allow(deprecated)]` dans le spike).

**Si le plan B est activé :** les contrôles passent par l'OSD de mpv (moins riches, thème limité, accessibilité réduite : pas d'annonces HTML) ; le clavier de la fenêtre est relayé par GTK ; la télécommande (API HTTP axum) reste entièrement fonctionnelle. Les écrans de `ui/tv` doivent alors se passer des contrôles HTML pendant la lecture : l'issue #64 prévoit les deux dispositions (plan A HTML, plan B OSD).

### Résultats mesurés (AMD Radeon 680M, 30 s de lecture, 1 à 4 runs par configuration)

| Configuration | Images perdues, total 30 s | Idem, régime établi (t=22..30 s) | CPU moyen t=2..5 s | Focus clavier | Bascules plein écran | Erreurs GL | Dérive RSS |
|---|---|---|---|---|---|---|---|
| A Wayland | 2,62 / 0,76 / 0,66 / 0,98 % | 0 (4 runs) | 24,9 à 26,1 % | 5/5 touches, dans le webview | 20, sans plantage | 0 | +16 Mo |
| A XWayland | 1,12 / 1,15 / 1,58 % | 0 (3 runs) | 26,5 à 27,0 % | 5/5, idem | 20, sans plantage | 0 | +25 à +26 Mo |
| A Wayland, DMABUF WebKit désactivé | 1,53 % (1 run) | 0 | 30,6 % | non vérifié | 20, sans plantage | 0 | +16 Mo |
| B Wayland | 0,88 / 0,66 / 0,77 % | 0 (3 runs) | 16,6 à 17,3 % | n/a (relais GTK vers mpv fonctionnel) | 20, sans plantage | 0 | +25 Mo |
| B XWayland | 0,84 % (1 run) | 0 | 18,7 % | n/a | 20, sans plantage | 0 | +22 Mo |
| Référence sans webview (Wayland) | 0,55 / 0,54 % | n/m | 16,0 / 16,6 % | n/a | n/m | n/m | +16 Mo |

n/a : non applicable ; n/m : non mesuré dans le tableau du rapport.

Réserves, reprises du rapport :

- Le run à froid de plan A Wayland (2,62 %) vient de 19 pertes sur 31 images dans la première seconde ; les runs suivants perdent 0 à 2 images au démarrage. Le premier lancement à froid perd davantage d'images.
- Le seuil < 1 % n'est donc tenu qu'en régime établi, pas toujours sur la fenêtre totale. Chaque pourcentage repose sur 5 à 25 images perdues : ne pas sur-interpréter l'écart Wayland / X11.
- La dérive RSS (environ +16 Mo) survient pendant les bascules puis reste plate, et elle existe aussi sans webview. 30 s ne prouvent ni l'absence ni la présence d'une fuite GL.
- `hwdec-current` vaut `vaapi` (zéro-copie) en Wayland et `vaapi-copy` en XWayland, sans surcoût CPU visible.
- Les captures ont été prises en dessinant l'arbre de widgets GTK (`gtk_widget_draw`) : elles prouvent la composition interne GTK, pas ce que le compositeur affiche. Elles interrompent aussi brièvement le rendu, ce qui explique très probablement la perte unique observée entre t=2 et 5 s (non démontré).
- Le focus a été testé avec des événements GTK synthétiques, pas au clavier physique.

### Risques et suivi

Responsable : EPIC 07, issue #57 (surface vidéo), avec #64 pour les contrôles. Ces vérifications ne sont pas faites ; l'ADR est acceptée en leur attente.

- **Confirmation visuelle à l'écran** (Wayland puis X11), clavier physique et souris (clic, survol) pendant la lecture, avant la clôture de #57.
- **Intel (iris, `vaapi`) et NVIDIA (`nvdec`/`vdpau`)** non mesurés ; risque d'écran noir avec le moteur DMABUF de WebKit sur NVIDIA (§5.2). Procédure du spike : `run-measure.sh`, trois runs par configuration.
- **Session X11 native** (GLX) : les runs « X11 » passent par XWayland.
- **HiDPI** (`scale_factor` > 1) : la taille de la `GLArea` est déjà multipliée par le facteur d'échelle, non testé.
- **Test de fuite** : 100 bascules plein écran (ou `--duration 300`), comparer `rss_kb_after_toggles` et `rss_kb_end`, surveiller `gl_errors`. Refaire les mesures sans `--screenshots`.
- **Déclencheurs de bascule vers le plan B** (si confirmés sur un autre GPU, ou dans le test réel) : contrôles invisibles ou fond opaque ; écran noir non résolu par `WEBKIT_DISABLE_DMABUF_RENDERER=1` ; focus clavier perdu ; pertes d'images régulières en régime établi. La bascule se fait alors par le réglage de démarrage, sans réécrire le rendu.
- Reporter ce résultat au §16 de `ARCHITECTURE.md` : l'hypothèse du §5.2 est confirmée sur AMD, non validée ailleurs.

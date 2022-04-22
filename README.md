# Projet Informatique Graphique

## Compilation

1. Vérifier que le projet a été cloné avec les sous-modules (`git submodule init --update` si jamais).
2. Installer la toolchain Rust [ici](https://rustup.rs)
3. Lancer les tests avec `cargo test` (il y en a un pour les courbes de Bézier)
4. Lancer la compilation avec `cargo build --bin <binary>` ou le lancement direct de l'application (après compilation
   automatique) avec `cargo run --bin <binary>`.

   `<binary>` est à choisir parmi :
    - `uv_sphere`: Démonstration du pipeline de rendu avec une sphère UV générée à la volée
    - `bezier`: Test interactif de courbes de Bézier en 2D
    - `bsurf`: Projet final, affichage d'une surface de bézier triangulée sur fond uni avec un éclairage trois points

   il est aussi possible d'ajouter l'option `--release` pour compiler le projet avec optimisations, ce qui améliore
   grandement les temps de chargements.

## Architecture

### Séparation bibliothèque/application

Le projet se divise en deux grandes parties - les abstractions OpenGL et les objets "haut-niveau" qui orchestrent le
rendu.
Rust se voulant un langage "safe", il est judicieux d'encapsuler les appels *unsafe* (dont les appels FFI en font parti)
derrière une API *safe*. Ceci est fait dans la bibliothèque disponible dans `violette/violette-low`.

`violette` encapsule via le mécanisme RAII (ce qui est beaucoup plus facile avec Rust qui n'a pas de *copy-constructor*
;))
la gestion des ressources GPU, et fournit un moyen de gérer les bindings OpenGL avec des types "gardes", ce qui permet à
l'API d'exposer leurs besoins en bindings à travers les signatures des fonctions (ceci équivaut aussi pour les types
haut-niveau). Par exemple, pour le rendu, un `Material` à besoin d'un binding de buffer de lumières pour pouvoir
exécuter
la phase de rendu en étant éclairé par celles-ci ; aussi, elle demande un Framebuffer actif qui sera la cible de la
phase de rendu. Ces gardes permettent de s'assurer que les ressources sont bien disponibles aux bons endroits quand on
en
a besoin. Aussi, les méthodes qui agissent sur ces bindings ne sont disponibles que lorsqu'un binding a été acquis.

De ce fait, tout le rendu propre au projet se fait via ces objets Rust, sans passer directement par les fonctions OpenGL
(qui sont considérées *unsafe*). Ceci s'accorde avec l'un des principes de Rust qui est de minimiser la quantité de code
unsafe écrit dans un projet.

Une limitation de l'architecture actuelle est qu'il est tout à fait possible de créer deux bindings pour deux ressources
d'un même type ; toutes les méthodes s'appliqueront sur le dernier binding créé, même si elles sont appelées avec le
binding précédent. Une factorisation du code (avec un objet de type contexte qui permettrait d'associer l'état des
bindings au *borrow checker* de Rust) permettrait d'empêcher la compilation de ce type de code.

### Bibliothèques externes

- `anyhow`: Gestion d'erreurs via le type `Result` de Rust ; permet de passer plus facilement les erreurs OpenGL
- `bytemuck` : Interface *safe* de manipulation de la mémoire (cast de valeurs en octets, notamment pour l'interfaçage
  GPU)
- `crevice` : Implémentation du layout `std140` automatique
- `duplicate` : Macro de duplication de code paramétrée
- `float-ord` : Implémentation d'un ordre total pour les flottants (par défaut Rust considère que INF/NaN ne sont pas
  orderable)
- `glam`: Algèbre linéaire
- `glutin`: Création et gestion du contexte OpenGL et de la fenêtre applicative
- `image` : Lecture de fichiers images et conversions de types (RGB8 vers RGB32F par exemple)
- `once_cell` : Types d'état global *safe*
- `num-derive`/`num-traits`: Programmation générique sur les scalaires, pour la généricité de certaines fonctions
  mathématiques et la conversions de type enums en scalaires
- `rand` : Génération de valeurs aléatoires (car la stdlib se veut minimaliste et n'inclue pas de module de ce type)
- `tracing`/`tracing-subscriber` : Instrumentation de code et logging structuré
- `thiserror` : Génération de types enum d'erreur

## Crédits

Textures lunaire: [SVI CGI Moon Kit](https://svs.gsfc.nasa.gov/cgi-bin/details.cgi?aid=4720)

Texture sol: [Texture Haven/Poly Haven (Rob Tuytel)](https://polyhaven.com/a/concrete_floor_painted)
# Prototype 7
All of this is very very WIP, so the readme may not stay up to date
## Usage
### Compiling
```bash
cargo build
```
### Launching
```bash
cargo run
```
### Launching with special level pattern
```bash
cargo run -- <path/to/file>
```
See examples in `./levels` and details in [Custom Levels](##Custom-Levels)

## Controls and gameplay
- Arrows to move
- Ctrl + arrow to place tower

### How the gameplay works
The player makes a move
Then the enemy plays, it walks towards the goal, and if it reaches it, it's joever. Enemies have HP, the towers deals 1 damage per shoot (for now).
Then the tower plays, for now the tower shoots in a straight line instantly and is blocked by the goal and rocks


## Custom Levels
The levels are customizable in files, see the folder `./levels` for ideas. For now the pattern is 1 tile = 2 characters; the first is the type of ground and the second is the content of the tile.

For the ground the choices are:
- `O` for grass (normal, walkable)
- `x` for water (non walkable but not an obstacle for towers' shoots)
- `|` for a path (walkable for enemies, is intended to be linked to the goal)

For the content of the tile:
- `-` for empty tile (default, nothing particular)
- `p` for the player (intended = only have one)
- `e` for enemies (should be placed on paths)
- `t` for towers
- `r` for rocks
- `g` for the goal (must have one)

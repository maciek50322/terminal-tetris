# Tetris in terminal

Learning rust project written with [ratatui](https://ratatui.rs/).

![Example image](example.png)

- Requires only terminal
- Game size is fit to initial terminal size. To set game size: set terminal size before starting the game.
- Gravity to right
- Shows block shadow at the end
- Controls
    - `Left` / `A` - rotate
    - `Right` / `D` - move forward
    - `Up` / `W` - move up
    - `Down` / `S` - move down
    - `Space` - move to the end (to shadow)
    - `P` - pause (click any control to resume)
    - `R` - Reset the game (only after finished)
    - `Ctrl + C` - exit

## Starting the game

For ready executables check out [Releases](https://github.com/maciek50322/terminal-tetris/releases).

Otherwise download rust and this project. Then
to download dependencies, build and run, use: 
```sh
cargo run --release
```
inside the project folder.
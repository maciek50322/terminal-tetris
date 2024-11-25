use ratatui::layout::Rect;
use std::io::{self};
use tetris::Tetris;

mod tetris;

fn main() -> io::Result<()> {
    let terminal = ratatui::init();
    let size = terminal.size().unwrap_or(ratatui::layout::Size {
        width: 40,
        height: 30,
    });
    let app = Tetris::new(
        Rect {
            width: size.width,
            height: size.height,
            x: 0,
            y: 0,
        },
        terminal,
    );
    let app_result = app.run();
    ratatui::restore();
    if let Err(ref error) = app_result {
        println!("{error}")
    }
    app_result
}

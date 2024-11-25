use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style, Stylize},
    text::{self, Text},
    widgets::{
        canvas::{Canvas, Painter},
        Block, Paragraph, Widget,
    },
    DefaultTerminal, Frame,
};
use std::{
    io,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tetris_block::TetrisBlock;

pub mod tetris_block;

#[derive(Debug, PartialEq, Eq)]
pub enum GameState {
    Playing,
    Paused,
    Finished,
}

#[derive(Debug)]
pub struct Tetris {
    cursor_state: bool,
    game_state: GameState,
    rounds: u64,
    points: u64,
    exit: bool,
    screen_rect: Rect,
    board_rect: Rect,
    info_rect: Vec<Rect>,
    next_rect: Rect,
    game_width: usize,
    game_height: usize,
    next_width: i32,
    next_height: i32,
    filled_area: Vec<Vec<Color>>,
    current_block: TetrisBlock,
    next_block: TetrisBlock,
    terminal: Arc<Mutex<DefaultTerminal>>,
    move_interval: Duration,
}

pub enum MoveDirection {
    Up,
    Down,
}

impl Tetris {
    pub fn new(mut screen_rect: Rect, terminal: DefaultTerminal) -> Self {
        if screen_rect.height < 10 {
            screen_rect.height = 10;
        }

        if screen_rect.width < 40 {
            screen_rect.width = 40;
        }

        let footer_height = 5;

        let board_rect = Rect {
            x: screen_rect.x,
            y: screen_rect.y,
            width: screen_rect.width,
            height: screen_rect.height - footer_height,
        };

        let next_rect = Rect {
            x: board_rect.x,
            y: board_rect.height,
            width: 11,
            height: footer_height,
        };

        let info_rect = vec![Rect {
            x: next_rect.x + next_rect.width,
            y: next_rect.y,
            width: 29,
            height: next_rect.height,
        }];

        let game_width = (board_rect.width - 2) as usize;
        let game_height = (board_rect.height - 2) as usize * 2;
        let next_width: i32 = (next_rect.width - 2) as i32;
        let next_height = (next_rect.height - 2) as i32 * 2;

        let filled_area = vec![vec![Color::Black; game_height]; game_width];

        let mut current_block = TetrisBlock::new_random();
        let mut next_block = TetrisBlock::new_random();
        current_block.pos = (
            0,
            game_height as i32 / 2 - current_block.pattern[0].len() as i32 / 2,
        );
        next_block.pos = (
            next_width / 2 - next_block.pattern.len() as i32 / 2,
            next_height / 2 - next_block.pattern[0].len() as i32 / 2,
        );

        Self {
            cursor_state: false,
            game_state: GameState::Playing,
            rounds: 0,
            points: 0,
            exit: false,
            screen_rect,
            next_rect,
            info_rect,
            board_rect,
            filled_area,
            game_width,
            game_height,
            next_width,
            next_height,
            current_block,
            next_block,
            terminal: Arc::new(Mutex::new(terminal)),
            move_interval: Duration::from_secs_f64(0.1),
        }
    }

    pub fn run(self) -> io::Result<()> {
        let atomic_terminal = Arc::clone(&self.terminal);
        let second_atomic_terminal = Arc::clone(&self.terminal);
        let atomic_self = Arc::new(Mutex::new(self));
        let second_atomic_self = Arc::clone(&atomic_self);

        let join_handle = thread::spawn(move || loop {
            let (parts, part_interval) = {
                let atomic_self = second_atomic_self.lock().unwrap();
                let parts = (atomic_self.move_interval.as_secs_f64() / 0.1).ceil() as u32;
                (parts, atomic_self.move_interval / parts)
            };

            for _ in 0..parts {
                thread::sleep(part_interval);
                let atomic_self = second_atomic_self.lock().unwrap();
                if atomic_self.exit {
                    return;
                };
            }

            {
                let mut atomic_self = second_atomic_self.lock().unwrap();
                if atomic_self.exit {
                    return;
                };

                if atomic_self.game_state == GameState::Finished {
                    return;
                }
                if atomic_self.game_state == GameState::Playing {
                    atomic_self.move_forward();
                    let _ = second_atomic_terminal
                        .lock()
                        .unwrap()
                        .draw(|frame| atomic_self.draw(frame));
                }
            }
        });

        while !{ atomic_self.lock().unwrap().exit } {
            {
                let mut atomic_self = atomic_self.lock().unwrap();
                atomic_terminal
                    .lock()
                    .unwrap()
                    .draw(|frame| atomic_self.draw(frame))?;
            }
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    atomic_self.lock().unwrap().handle_key_event(key_event)?
                }
                _ => {}
            };
        }

        join_handle.join().unwrap();

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> io::Result<()> {
        match key_event.code {
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit();
            }
            KeyCode::Left | KeyCode::Char('a') => self.rotate90(),
            KeyCode::Right | KeyCode::Char('d') => self.move_forward(),
            KeyCode::Up | KeyCode::Char('w') => self.move_side(MoveDirection::Up),
            KeyCode::Down | KeyCode::Char('s') => self.move_side(MoveDirection::Down),
            KeyCode::Char(' ') => self.move_till_end(),
            KeyCode::Char('p') => self.pause(),
            _ => {}
        }
        Ok(())
    }

    fn pause(&mut self) {
        match self.game_state {
            GameState::Playing => self.game_state = GameState::Paused,
            GameState::Paused => self.game_state = GameState::Playing,
            _ => {}
        };
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn finish_round(&mut self) {
        let (x, y) = self.current_block.pos;
        let x = x as usize;
        let y = y as usize;

        let mut cleared_cols = 0;
        for (i, col) in self.current_block.pattern.iter().enumerate() {
            for (j, draw) in col.iter().enumerate() {
                if *draw {
                    self.filled_area[x + i][y + j] = self.current_block.color;
                }
            }
            if self.filled_area[x + i].iter().all(|c| *c != Color::Black) {
                self.filled_area[x + i]
                    .iter_mut()
                    .for_each(|x| *x = Color::Black);
                self.filled_area[..x + i + 1].rotate_right(1);
                cleared_cols += 1;
            }
        }

        self.points += match cleared_cols {
            1 => 40,
            2 => 100,
            3 => 300,
            4 => 1200,
            _ => 0,
        };

        self.rounds += 1;

        let starting_y_pos = (self.current_block.pos.1 as usize).min(
            self.game_height
                - self
                    .next_block
                    .pattern
                    .iter()
                    .map(|x| x.len())
                    .max()
                    .unwrap_or(0),
        );

        for (i, col) in self.next_block.pattern.iter().enumerate() {
            for (j, draw) in col.iter().enumerate() {
                if *draw && self.filled_area[i][j + starting_y_pos] != Color::Black {
                    // game lost
                    self.game_state = GameState::Finished;
                    return;
                }
            }
        }

        self.current_block = std::mem::replace(&mut self.next_block, TetrisBlock::new_random());
        self.current_block.pos = (0, starting_y_pos as i32);
        self.next_block.pos = (
            self.next_width / 2 - self.next_block.pattern.len() as i32 / 2,
            self.next_height / 2 - self.next_block.pattern[0].len() as i32 / 2,
        );
    }

    fn rotate90(&mut self) {
        if self.game_state == GameState::Finished {
            return;
        }
        if self.game_state == GameState::Paused {
            self.game_state = GameState::Playing
        }

        let new_pattern = TetrisBlock::rotate90(&self.current_block.pattern);

        let (x, y) = self.current_block.pos;
        let x = x as usize;
        let y = y as usize;

        for move_y in 0..4.min(y).max(1) {
            let y = y - move_y;
            let mut can_rotate = true;

            for (i, col) in new_pattern.iter().enumerate() {
                for (j, draw) in col.iter().enumerate() {
                    if *draw
                        && (x + i >= self.game_width
                            || y + j >= self.game_height
                            || self.filled_area[x + i][y + j] != Color::Black)
                    {
                        can_rotate = false;
                        break;
                    }
                }
                if !can_rotate {
                    break;
                }
            }

            if can_rotate {
                self.current_block.pattern = new_pattern;
                self.current_block.pos.1 -= move_y as i32;
                return;
            }
        }
    }

    fn move_forward(&mut self) {
        if self.game_state == GameState::Finished {
            return;
        }
        if self.game_state == GameState::Paused {
            self.game_state = GameState::Playing
        }

        let (x, y) = self.current_block.pos;
        let x = x as usize;
        let y = y as usize;
        for (i, col) in self.current_block.pattern.iter().enumerate() {
            for (j, draw) in col.iter().enumerate() {
                if *draw
                    && (x + i + 1 >= self.filled_area.len()
                        || self.filled_area[x + i + 1][y + j] != Color::Black)
                {
                    // stop, next move
                    self.finish_round();
                    return;
                }
            }
        }
        self.current_block.pos.0 += 1;
    }

    fn get_end_move_pos(&self) -> (i32, i32) {
        let (x, y) = self.current_block.pos;
        let mut x: usize = x as usize;
        let y = y as usize;
        loop {
            for (i, col) in self.current_block.pattern.iter().enumerate() {
                for (j, draw) in col.iter().enumerate() {
                    if *draw
                        && (x + i + 1 >= self.filled_area.len()
                            || self.filled_area[x + i + 1][y + j] != Color::Black)
                    {
                        // stop
                        return (x as i32, y as i32);
                    }
                }
            }
            x += 1;
        }
    }

    fn move_till_end(&mut self) {
        if self.game_state == GameState::Finished {
            return;
        }
        if self.game_state == GameState::Paused {
            self.game_state = GameState::Playing
        }

        self.current_block.pos.0 = self.get_end_move_pos().0;
        self.finish_round();
    }

    fn move_side(&mut self, direction: MoveDirection) {
        if self.game_state == GameState::Finished {
            return;
        }
        if self.game_state == GameState::Paused {
            self.game_state = GameState::Playing
        }

        let (x, y) = self.current_block.pos;
        let x = x as usize;
        let y = y as usize;
        let direction = match direction {
            MoveDirection::Down => -1,
            MoveDirection::Up => 1,
        };
        for (i, col) in self.current_block.pattern.iter().enumerate() {
            for (j, draw) in col.iter().enumerate() {
                if *draw
                    && ((y + j == 0 && direction < 0)
                        || (y + j + 1 == self.game_height && direction > 0)
                        || self.filled_area[x + i][(y as i32 + j as i32 + direction) as usize]
                            != Color::Black)
                {
                    // can't move there
                    return;
                }
            }
        }
        self.current_block.pos.1 += direction;
    }
}

impl Widget for &mut Tetris {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let last_point_exists = buf
            .cell((self.screen_rect.width - 1, self.screen_rect.height - 1))
            .is_some();

        if last_point_exists {
            let next = Canvas::default()
                .block(
                    Block::bordered()
                        .bold()
                        .title_top(" Next ".bold().green())
                        .title_bottom(" <Space> ".bold().blue())
                        .title_alignment(Alignment::Center),
                )
                .background_color(Color::Black)
                .marker(ratatui::symbols::Marker::HalfBlock)
                .x_bounds([-1.0, self.next_width as f64 - 1.0])
                .y_bounds([0.0, self.next_height as f64])
                .paint(|ctx| {
                    ctx.layer();
                    ctx.draw(&self.next_block);
                });

            next.render(self.next_rect, buf);

            let info = Paragraph::new(Text::from(vec![
                text::Line::from(vec![
                    " Score: ".white(),
                    self.points.to_string().bold().green(),
                ]),
                text::Line::from(vec![
                    " Round: ".white(),
                    self.rounds.to_string().bold().blue(),
                ]),
                text::Line::from(vec![
                    " State: ".white(),
                    match self.game_state {
                        GameState::Playing => "Playing".to_string().green(),
                        GameState::Paused => "Paused".to_string().yellow().bold(),
                        GameState::Finished => "Finished".to_string().red().bold(),
                    },
                ]),
            ]))
            .block(
                Block::bordered()
                    .title_top(" Info ".bold().green())
                    .title_bottom(
                        " <Ctrl + C>".bold().blue()
                            + " Exit ".not_bold().white()
                            + "<P>".bold().blue()
                            + " Pause ".not_bold().white(),
                    )
                    .title_alignment(Alignment::Center),
            );

            info.render(self.info_rect[0], buf);

            let board = Canvas::default()
                .block(
                    Block::bordered()
                        .bold()
                        .fg(Color::Gray)
                        .title_top(" Tetris ".bold().green())
                        .title_bottom(
                            " <A/←>".bold().blue()
                                + " Rotate ".white().not_bold()
                                + "<W/↑, S/↓, D/→>".bold().blue()
                                + " Move ".white().not_bold(),
                        )
                        .title_alignment(Alignment::Center),
                )
                .background_color(Color::Black)
                .marker(ratatui::symbols::Marker::HalfBlock)
                .x_bounds([-1.0, self.game_width as f64 - 1.0])
                .y_bounds([0.0, self.game_height as f64])
                .paint(|ctx| {
                    ctx.layer();

                    let mut painter = Painter::from(&mut *ctx);
                    for (x, col) in self.filled_area.iter().enumerate() {
                        for (y, color) in col.iter().enumerate() {
                            if *color != Color::Black {
                                if let Some((x, y)) = painter.get_point(x as f64, y as f64) {
                                    painter.paint(x, y, *color);
                                }
                            }
                        }
                    }

                    let mut last_pos = self.current_block.clone();
                    last_pos.pos = self.get_end_move_pos();
                    last_pos.color = Color::DarkGray;
                    ctx.draw(&last_pos);

                    ctx.draw(&self.current_block);
                });

            board.render(self.board_rect, buf);

            // removes cursor from inside of the game
            // has to update each render to actually move cursor there
            // has to be rendered last on screen so there's cursor isn't left inside board after render
            // has to write 1 before last character on screen, so cursor going to next char doesn't go to next line
            self.cursor_state = !self.cursor_state;
            buf.cell_mut((self.screen_rect.width - 2, self.screen_rect.height - 1))
                .unwrap()
                .set_fg(if self.cursor_state {
                    Color::Black
                } else {
                    Color::Reset
                });
        } else {
            if self.game_state == GameState::Playing {
                self.game_state = GameState::Paused;
            }

            if area.height < 1 {
                panic!("{}", area.height);
            }

            if area.height > 0 {
                buf.set_string(0, 0, "Terminal too small", Style::new().bold());
            }
            if area.height > 1 {
                buf.set_string(
                    0,
                    1,
                    format!(
                        "Expected at least ( {} x {} )",
                        self.screen_rect.width, self.screen_rect.height
                    ),
                    Style::new(),
                );
            }
            if area.height > 2 {
                buf.set_string(
                    0,
                    2,
                    format!("Current size ( {} x {} )", area.width, area.height),
                    Style::new(),
                );
            }
        }
    }
}

use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::{
    Attribute, Color, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::QueueableCommand;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use rodio::{Decoder, OutputStream, Sink, Source};

struct CleanUp;

impl Drop for CleanUp {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = stdout.queue(Show);
        let _ = stdout.queue(ResetColor);
        let _ = stdout.queue(LeaveAlternateScreen);
        let _ = stdout.flush();
        let _ = disable_raw_mode();
    }
}

fn play_theme_audio_loop(running: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let mut possible_paths = vec![
            PathBuf::from("assets/Hirako_Shinji_Theme.mp3"),
            PathBuf::from("/Users/adolfcodler/Programming/cli_tools/Sakanade/assets/Hirako_Shinji_Theme.mp3"),
        ];

        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                possible_paths.push(parent.join("assets/Hirako_Shinji_Theme.mp3"));
                if let Some(parent2) = parent.parent() {
                    possible_paths.push(parent2.join("assets/Hirako_Shinji_Theme.mp3"));
                    if let Some(parent3) = parent2.parent() {
                        possible_paths.push(parent3.join("assets/Hirako_Shinji_Theme.mp3"));
                    }
                }
            }
        }

        let mut audio_file = None;
        for path in &possible_paths {
            if path.exists() {
                audio_file = Some(path.clone());
                break;
            }
        }

        if let Some(path) = audio_file {
            let Ok((_stream, stream_handle)) = OutputStream::try_default() else {
                return;
            };
            let Ok(sink) = Sink::try_new(&stream_handle) else {
                return;
            };

            while running.load(Ordering::Relaxed) {
                if let Ok(file) = File::open(&path) {
                    let reader = BufReader::new(file);
                    if let Ok(source) = Decoder::new(reader) {
                        sink.append(source.repeat_infinite());
                        while running.load(Ordering::Relaxed) && !sink.empty() {
                            std::thread::sleep(Duration::from_millis(200));
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    });
}

fn convert_color(c: vt100::Color) -> Option<Color> {
    match c {
        vt100::Color::Default => None,
        vt100::Color::Idx(idx) => Some(Color::AnsiValue(idx)),
        vt100::Color::Rgb(r, g, b) => Some(Color::Rgb { r, g, b }),
    }
}

fn reverse_arrow_key(key: KeyCode) -> KeyCode {
    match key {
        KeyCode::Up => KeyCode::Down,
        KeyCode::Down => KeyCode::Up,
        KeyCode::Left => KeyCode::Right,
        KeyCode::Right => KeyCode::Left,
        other => other,
    }
}

fn key_event_to_bytes(key: KeyEvent) -> Vec<u8> {
    let inverted_code = reverse_arrow_key(key.code);

    match inverted_code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                let ascii = (c as u8).to_ascii_lowercase();
                if (b'a'..=b'z').contains(&ascii) {
                    vec![ascii - b'a' + 1]
                } else if c == '[' {
                    vec![0x1B]
                } else if c == '\\' {
                    vec![0x1C]
                } else if c == ']' {
                    vec![0x1D]
                } else if c == '^' {
                    vec![0x1E]
                } else if c == '_' {
                    vec![0x1F]
                } else {
                    c.to_string().into_bytes()
                }
            } else if key.modifiers.contains(KeyModifiers::ALT) {
                let mut bytes = vec![0x1B];
                bytes.extend(c.to_string().as_bytes());
                bytes
            } else {
                c.to_string().into_bytes()
            }
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7F],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::BackTab => vec![0x1B, b'[', b'Z'],
        KeyCode::Esc => vec![0x1B],
        KeyCode::Up => vec![0x1B, b'[', b'A'],
        KeyCode::Down => vec![0x1B, b'[', b'B'],
        KeyCode::Right => vec![0x1B, b'[', b'C'],
        KeyCode::Left => vec![0x1B, b'[', b'D'],
        KeyCode::Home => vec![0x1B, b'[', b'H'],
        KeyCode::End => vec![0x1B, b'[', b'F'],
        KeyCode::PageUp => vec![0x1B, b'[', b'5', b'~'],
        KeyCode::PageDown => vec![0x1B, b'[', b'6', b'~'],
        KeyCode::Delete => vec![0x1B, b'[', b'3', b'~'],
        KeyCode::Insert => vec![0x1B, b'[', b'2', b'~'],
        KeyCode::F(n) => match n {
            1 => vec![0x1B, b'O', b'P'],
            2 => vec![0x1B, b'O', b'Q'],
            3 => vec![0x1B, b'O', b'R'],
            4 => vec![0x1B, b'O', b'S'],
            5 => vec![0x1B, b'[', b'1', b'5', b'~'],
            6 => vec![0x1B, b'[', b'1', b'7', b'~'],
            7 => vec![0x1B, b'[', b'1', b'8', b'~'],
            8 => vec![0x1B, b'[', b'1', b'9', b'~'],
            9 => vec![0x1B, b'[', b'2', b'0', b'~'],
            10 => vec![0x1B, b'[', b'2', b'1', b'~'],
            11 => vec![0x1B, b'[', b'2', b'3', b'~'],
            12 => vec![0x1B, b'[', b'2', b'4', b'~'],
            _ => vec![],
        },
        _ => vec![],
    }
}

fn draw_inverted_screen(
    parser: &vt100::Parser,
    stdout: &mut io::Stdout,
    rows: u16,
    cols: u16,
) -> Result<()> {
    let screen = parser.screen();
    stdout.queue(Hide)?;

    for r in 0..rows {
        // Inverted row: top to bottom reversed
        let src_row = rows.saturating_sub(1).saturating_sub(r);
        stdout.queue(MoveTo(0, r))?;

        let mut current_fg: Option<Color> = None;
        let mut current_bg: Option<Color> = None;
        let mut current_bold = false;
        let mut current_italic = false;
        let mut current_underline = false;
        let mut current_inverse = false;

        for c in 0..cols {
            // Inverted col: left to right reversed
            let src_col = cols.saturating_sub(1).saturating_sub(c);
            let cell = screen.cell(src_row, src_col);

            let (ch, fg, bg, bold, italic, underline, inverse) = match cell {
                Some(cell) => {
                    let ch = cell.contents();
                    let ch_char = ch.chars().next().unwrap_or(' ');
                    let fg = convert_color(cell.fgcolor());
                    let bg = convert_color(cell.bgcolor());
                    let bold = cell.bold();
                    let italic = cell.italic();
                    let underline = cell.underline();
                    let inverse = cell.inverse();
                    (ch_char, fg, bg, bold, italic, underline, inverse)
                }
                None => (' ', None, None, false, false, false, false),
            };

            if fg != current_fg {
                match fg {
                    Some(color) => stdout.queue(SetForegroundColor(color))?,
                    None => stdout.queue(SetForegroundColor(Color::Reset))?,
                };
                current_fg = fg;
            }

            if bg != current_bg {
                match bg {
                    Some(color) => stdout.queue(SetBackgroundColor(color))?,
                    None => stdout.queue(SetBackgroundColor(Color::Reset))?,
                };
                current_bg = bg;
            }

            if bold != current_bold {
                if bold {
                    stdout.queue(SetAttribute(Attribute::Bold))?;
                } else {
                    stdout.queue(SetAttribute(Attribute::NormalIntensity))?;
                }
                current_bold = bold;
            }

            if italic != current_italic {
                if italic {
                    stdout.queue(SetAttribute(Attribute::Italic))?;
                } else {
                    stdout.queue(SetAttribute(Attribute::NoItalic))?;
                }
                current_italic = italic;
            }

            if underline != current_underline {
                if underline {
                    stdout.queue(SetAttribute(Attribute::Underlined))?;
                } else {
                    stdout.queue(SetAttribute(Attribute::NoUnderline))?;
                }
                current_underline = underline;
            }

            if inverse != current_inverse {
                if inverse {
                    stdout.queue(SetAttribute(Attribute::Reverse))?;
                } else {
                    stdout.queue(SetAttribute(Attribute::NoReverse))?;
                }
                current_inverse = inverse;
            }

            let printable = if ch.is_control() { ' ' } else { ch };
            write!(stdout, "{}", printable)?;
        }
    }

    // Invert cursor position
    if !screen.hide_cursor() {
        let (cursor_r, cursor_c) = screen.cursor_position();
        let target_r = rows.saturating_sub(1).saturating_sub(cursor_r);
        let target_c = cols.saturating_sub(1).saturating_sub(cursor_c);
        stdout.queue(MoveTo(target_c, target_r))?;
        stdout.queue(Show)?;
    }

    stdout.flush()?;
    Ok(())
}

fn main() -> Result<()> {
    let _cleanup = CleanUp;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let shell = std::env::var("SHELL").unwrap_or_else(|_| String::from("/bin/zsh"));
    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .context("Failed to open pty")?;

    let mut cmd = if args.is_empty() {
        CommandBuilder::new(&shell)
    } else {
        let mut builder = CommandBuilder::new(&args[0]);
        for arg in &args[1..] {
            builder.arg(arg);
        }
        builder
    };
    cmd.env("TERM", "xterm-256color");
    let mut child = pair.slave.spawn_command(cmd).context("Failed to spawn process in PTY")?;
    drop(pair.slave);

    let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 0)));
    let running = Arc::new(AtomicBool::new(true));

    // Start Shinji Hirako's Theme music on loop
    play_theme_audio_loop(Arc::clone(&running));

    let mut pty_reader = pair.master.try_clone_reader().context("Failed to clone PTY reader")?;
    let mut pty_writer = pair.master.take_writer().context("Failed to take PTY writer")?;

    // Thread: Read PTY output -> feed to vt100 virtual screen
    let parser_clone = Arc::clone(&parser);
    let running_clone = Arc::clone(&running);
    let pty_thread = std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        while running_clone.load(Ordering::Relaxed) {
            match pty_reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if let Ok(mut p) = parser_clone.lock() {
                        p.process(&buf[..n]);
                    }
                }
                Err(_) => break,
            }
        }
        running_clone.store(false, Ordering::Relaxed);
    });

    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    stdout.queue(EnterAlternateScreen)?;
    stdout.queue(Hide)?;
    stdout.flush()?;

    let mut current_rows = rows;
    let mut current_cols = cols;

    while running.load(Ordering::Relaxed) {
        if let Ok(Some(status)) = child.try_wait() {
            let _ = status;
            running.store(false, Ordering::Relaxed);
            break;
        }

        // Process terminal user inputs
        while event::poll(Duration::from_millis(5))? {
            match event::read()? {
                Event::Key(key) => {
                    // Control-Q to safely exit Sakanade
                    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
                        running.store(false, Ordering::Relaxed);
                        break;
                    }

                    let bytes = key_event_to_bytes(key);
                    if !bytes.is_empty() {
                        let _ = pty_writer.write_all(&bytes);
                        let _ = pty_writer.flush();
                    }
                }
                Event::Resize(w, h) => {
                    current_cols = w;
                    current_rows = h;
                    let _ = pair.master.resize(PtySize {
                        rows: h,
                        cols: w,
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                    if let Ok(mut p) = parser.lock() {
                        p.screen_mut().set_size(h, w);
                    }
                }
                _ => {}
            }
        }

        if !running.load(Ordering::Relaxed) {
            break;
        }

        // Render current virtual terminal inverted (top to bottom reversed, left to right reversed)
        if let Ok(p) = parser.lock() {
            let _ = draw_inverted_screen(&p, &mut stdout, current_rows, current_cols);
        }

        std::thread::sleep(Duration::from_millis(15));
    }

    let _ = child.kill();
    let _ = pty_thread.join();

    Ok(())
}


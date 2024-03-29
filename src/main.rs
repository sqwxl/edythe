use std::fs::File;
use std::io::{BufWriter, Stdout, Write};
use std::path::PathBuf;

use clap::Parser;
use crossterm::{cursor, queue};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::Print,
    terminal,
};

#[derive(Debug)]
enum BufferPath {
    File(PathBuf),
    Temp(usize),
}

struct Buffer {
    path: BufferPath,
    data: String,
}
impl Buffer {
    fn new(path: BufferPath, data: String) -> Self {
        Self { path, data }
    }

    fn append_char(&mut self, c: char) {
        self.data.push(c);
    }

    fn delete_char_from_end(&mut self) {
        if !self.data.is_empty() {
            self.data.pop();
        }
    }
}

struct Editor {
    buffer: Buffer,
}
impl Editor {
    fn new(buffer: Buffer) -> Editor {
        Editor { buffer }
    }

    fn save_to_disk(&self) -> std::io::Result<()> {
        if let BufferPath::File(ref file_path) = self.buffer.path {
            let mut f = BufWriter::new(File::create(file_path)?);
            f.write(self.buffer.data.as_bytes())?;
        }

        Ok(())
    }

    fn insert_char(&mut self, c: char) {
        self.buffer.append_char(c);
    }

    fn delete_last_char(&mut self) {
        self.buffer.delete_char_from_end();
    }
}

#[derive(Debug)]
enum EditorEvent {
    Edited,
    Quit,
    Continue,
}

struct Tui {
    out: Stdout,
    editor: Editor,
}

impl Tui {
    fn new(editor: Editor) -> Self {
        Self {
            // Crossterm is can write to any buffer that is `Write`, in our case, that's just stdout
            out: std::io::stdout(),
            editor,
        }
    }

    fn run(&mut self) {
        // The "alternate screen" is like another window or tab that you can draw to. When it's closed
        // the user is returned to the regular shell prompt. This is how "full-screen" terminal apps
        // like vim or htop do it.
        execute!(&self.out, terminal::EnterAlternateScreen).unwrap();

        // By default the terminal acts sort of like the default text input of the shell. By enabling
        // "raw mode" crossterm gives us full control of what and how stuff gets displayed.
        terminal::enable_raw_mode().unwrap();

        // first draw
        self.draw();
        // This is the main loop our app runs in.
        loop {
            match self.read_input() {
                EditorEvent::Continue => continue,
                EditorEvent::Quit => break,
                EditorEvent::Edited => {
                    self.draw();
                }
            };
        }

        terminal::disable_raw_mode().unwrap();
        execute!(&self.out, terminal::LeaveAlternateScreen).unwrap();
    }

    fn draw(&mut self) {
        queue!(
            &mut self.out,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0),
        )
        .unwrap();

        let mut lines = self.editor.buffer.data.lines();

        // print the first line
        queue!(&mut self.out, Print(lines.next().unwrap_or(""))).unwrap();

        // reset the cursor before each subsequent line
        for line in lines {
            queue!(&self.out, cursor::MoveToNextLine(1), Print(line),).unwrap();
        }

        self.out.flush().unwrap();
    }

    fn read_input(&mut self) -> EditorEvent {
        match event::read().unwrap() {
            Event::Key(key_event) => self.match_keyevent(key_event),
            Event::Resize(_, _) => EditorEvent::Continue, // TODO
            Event::Mouse(_) => EditorEvent::Continue,     // TODO
            _ => EditorEvent::Continue,
        }
    }

    fn match_keyevent(&mut self, key_event: KeyEvent) -> EditorEvent {
        match key_event {
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => return EditorEvent::Quit,
            KeyEvent {
                code: KeyCode::Char('s'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => self
                .editor
                .save_to_disk()
                .expect("I couldn't save the file for some reason."),
            KeyEvent {
                code: KeyCode::Backspace,
                ..
            } => self.editor.delete_last_char(),
            KeyEvent {
                code: KeyCode::Char(c),
                ..
            } => self.editor.insert_char(c),
            _ => return EditorEvent::Continue,
        }

        EditorEvent::Edited
    }
}

// Define the command line arguments
#[derive(Parser)]
struct Args {
    #[arg()]
    file: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    let buffer = match args.file {
        Some(path) => {
            // read file content into buffer; or empty string if the file doesn't exist
            let data = std::fs::read_to_string(&path).unwrap_or_default();

            Buffer::new(BufferPath::File(path), data)
        }
        None => Buffer {
            path: BufferPath::Temp(0),
            data: String::new(),
        },
    };

    let editor = Editor::new(buffer);

    let mut tui = Tui::new(editor);

    tui.run();
}

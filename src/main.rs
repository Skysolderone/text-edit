// use crossterm::event::*;
// // use crossterm::event::{Event, KeyCode, KeyEvent};
// use crossterm::style::*;
// use crossterm::terminal::ClearType;
// use crossterm::{cursor, event, execute, queue, style, terminal};
// use std::cmp::Ordering;

// use std::io::ErrorKind;
// use std::io::{self, stdout, Write};
// use std::path::PathBuf;

// use std::time::{Duration, Instant};

// use std::{cmp, env, fs};
use crossterm::event::*;
use crossterm::style::*;
use crossterm::terminal::ClearType;
use crossterm::{cursor, event, execute, queue, style, terminal};
use std::cmp::Ordering;
use std::io::{stdout, ErrorKind, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::{cmp, env, fs, io};

const TAB_STOP: usize = 8;
const QUIT_TIMES: u8 = 3;

#[derive(Clone, Copy)]
enum HighlightType {
    Normal,
    Number,
    SearchMatch,
    String,
    CharLiteral,
    Comment,
    Other(Color),
    MutilComment,
}
trait SyntaxHighlight {
    fn mutil_comment(&self) -> Option<(&str, &str)>;
    fn comment_start(&self) -> &str;
    fn file_type(&self) -> &str;
    fn extensions(&self) -> &[&str];
    fn update_syntax(&self, at: usize, editor_rows: &mut Vec<Row>);
    fn syntax_color(&self, highlight_type: &HighlightType) -> Color;
    fn color_row(&self, render: &str, highlight_type: &[HighlightType], out: &mut EditContents) {
        // let mut current_color = self.syntax_color(&HighlightType::Normal);
        // render.chars().enumerate().for_each(|(i, c)| {
        //     let _ = queue!(
        //         out,
        //         SetForegroundColor(self.syntax_color(&highlight_type[i]))
        //     );
        //     out.push(c);
        //     let _ = queue!(out, ResetColor);
        // });
        // render.char_indices().for_each(|(i, c)| {
        //     let color = self.syntax_color(&highlight_type[i]);
        //     if current_color != color {
        //         current_color = color;
        //         let _ = queue!(out, SetForegroundColor(color));
        //     }
        //     out.push(c);
        // });
        // let _ = queue!(out, SetForegroundColor(Color::Reset));
        render.chars().enumerate().for_each(|(i, c)| {
            let _ = queue!(
                out,
                SetForegroundColor(self.syntax_color(&highlight_type[i]))
            );
            out.push(c);
            let _ = queue!(out, ResetColor);
        });
    }
    fn is_separator(&self, c: char) -> bool {
        c.is_whitespace()
            || [
                ',', '.', '(', ')', '+', '-', '/', '*', '=', '~', '%', '<', '>', '"', '\'', ';',
                '&',
            ]
            .contains(&c)
    }
}
enum SearchDirection {
    Forward,
    Backward,
}
struct SearchIndex {
    x_index: usize,
    y_index: usize,
    x_direction: Option<SearchDirection>,
    y_direction: Option<SearchDirection>,
    previous_heiglight: Option<(usize, Vec<HighlightType>)>,
}
impl SearchIndex {
    fn new() -> Self {
        Self {
            x_index: 0,
            y_index: 0,
            x_direction: None,
            y_direction: None,
            previous_heiglight: None,
        }
    }
    fn reset(&mut self) {
        self.y_index = 0;
        self.x_index = 0;
        self.x_direction = None;
        self.y_direction = None;
        self.previous_heiglight = None;
    }
}
struct StatusMessage {
    message: Option<String>,
    set_time: Option<Instant>,
}
impl StatusMessage {
    fn new(initial_msg: String) -> Self {
        Self {
            message: Some(initial_msg),
            set_time: None,
        }
    }
    fn set_message(&mut self, message: String) {
        self.message = Some(message);
        self.set_time = Some(Instant::now())
    }
    fn message(&mut self) -> Option<&String> {
        self.set_time.and_then(|time| {
            if time.elapsed() > Duration::from_secs(5) {
                self.message = None;
                self.set_time = None;
                None
            } else {
                Some(self.message.as_ref().unwrap())
            }
        })
    }
}
#[derive(Default)]
struct Row {
    row_content: String,
    render: String,
    hithlight: Vec<HighlightType>,
    is_comment: bool,
}

impl Row {
    fn new(row_content: String, render: String) -> Self {
        Self {
            row_content,
            render,
            hithlight: Vec::new(),
            is_comment: false,
        }
    }
    fn get_row_content_x(&self, render_x: usize) -> usize {
        let mut current_render_x = 0;
        for (cursor_x, ch) in self.row_content.chars().enumerate() {
            if ch == '\t' {
                current_render_x += (TAB_STOP - 1) - (current_render_x % TAB_STOP);
            }
            current_render_x += 1;
            if current_render_x > render_x {
                return cursor_x;
            }
        }
        0
    }
    fn insert_char(&mut self, at: usize, ch: char) {
        self.row_content.insert(at, ch);
        EditRows::render_row(self);
    }
    fn delete_char(&mut self, at: usize) {
        self.row_content.remove(at);
        EditRows::render_row(self);
    }
}
struct EditRows {
    row_contents: Vec<Row>,
    filename: Option<PathBuf>,
}
impl EditRows {
    fn new(syntax_hight: &mut Option<Box<dyn SyntaxHighlight>>) -> Self {
        // let mut arg = env::args();
        match env::args().nth(1) {
            None => Self {
                row_contents: Vec::new(),
                filename: None,
            },
            Some(file) => Self::from_file(file.into(), syntax_hight),
        }
        // Self {
        //     row_contents: vec!["Hello world".into()],
        // }
    }
    fn join_adjacent_row(&mut self, at: usize) {
        let current_row = self.row_contents.remove(at);
        let previous_row = self.get_editor_row_mut(at - 1);
        previous_row.row_content.push_str(&current_row.row_content);
        Self::render_row(previous_row);
    }
    fn save(&self) -> io::Result<usize> {
        match &self.filename {
            None => Err(io::Error::new(ErrorKind::Other, "no file name specifted")),
            Some(name) => {
                let mut file = fs::OpenOptions::new().write(true).create(true).open(name)?;
                let contents: String = self
                    .row_contents
                    .iter()
                    .map(|it| it.row_content.as_str())
                    .collect::<Vec<&str>>()
                    .join("\n");
                file.set_len(contents.len() as u64)?;
                file.write_all(contents.as_bytes());
                Ok(contents.as_bytes().len())
            }
        }
    }
    fn get_editor_row_mut(&mut self, at: usize) -> &mut Row {
        &mut self.row_contents[at]
    }
    fn from_file(file: PathBuf, syntax_high: &mut Option<Box<dyn SyntaxHighlight>>) -> Self {
        let file_contents = fs::read_to_string(&file).expect("Unable to read file");
        let mut row_contents = Vec::new();
        file.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| Output::select_syntax(ext).map(|syntax| syntax_high.insert(syntax)));

        // file.extension()
        //     .and_then(|ext| ext.to_str())
        //     .map(|ext| Output::select_syntax(ext).map(|syntax| syntax_high.insert(syntax)));

        file_contents.lines().enumerate().for_each(|(i, line)| {
            let mut row = Row::new(line.into(), String::new());
            Self::render_row(&mut row);
            row_contents.push(row);
            if let Some(it) = syntax_high {
                it.update_syntax(i, &mut row_contents)
            }
        });
        Self {
            filename: Some(file),
            // row_contents: file_contents
            //     .lines()
            //     .map(|it| {
            //         let mut row = Row::new(it.into(), String::new());
            //         Self::render_row(&mut row);
            //         row
            //     })
            //     .collect(),
            row_contents,
        }
    }
    fn insert_char(&mut self, at: usize, contents: String) {
        let mut new_row = Row::new(contents, String::new());
        Self::render_row(&mut new_row);
        self.row_contents.insert(at, new_row);
    }
    fn num_of_rows(&self) -> usize {
        self.row_contents.len()
    }
    fn get_row(&self, at: usize) -> &str {
        &self.row_contents[at].row_content
    }
    fn render_row(row: &mut Row) {
        let mut index = 0;
        let capacity = row
            .row_content
            .chars()
            .fold(0, |acc, next| acc + if next == '\t' { TAB_STOP } else { 1 });
        row.render = String::with_capacity(capacity);
        row.row_content.chars().for_each(|c| {
            index += 1;
            if c == '\t' {
                row.render.push(' ');
                while index % TAB_STOP != 0 {
                    row.render.push(' ');
                    index += 1
                }
            } else {
                row.render.push(c);
            }
        });
    }
    fn get_render(&self, at: usize) -> &String {
        &self.row_contents[at].render
    }
    fn get_editer_row(&self, at: usize) -> &Row {
        &self.row_contents[at]
    }
}
struct EditContents {
    content: String,
}
impl EditContents {
    fn new() -> Self {
        Self {
            content: String::new(),
        }
    }
    fn push(&mut self, ch: char) {
        self.content.push(ch)
    }
    fn push_str(&mut self, string: &str) {
        self.content.push_str(string)
    }
}
impl io::Write for EditContents {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.content.push_str(s);
                Ok(s.len())
            }
            Err(_) => Err(io::ErrorKind::WriteZero.into()),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        let out = write!(stdout(), "{}", self.content);
        stdout().flush()?;
        self.content.clear();
        out
    }
}
#[derive(Clone, Copy)]
struct CursorController {
    cursor_x: usize,
    cursor_y: usize,
    screen_columns: usize,
    sreen_raws: usize,
    row_offset: usize,
    column_size: usize,
    render_x: usize,
}
impl CursorController {
    fn new(win_size: (usize, usize)) -> CursorController {
        Self {
            cursor_x: 0,
            cursor_y: 0,
            screen_columns: win_size.0,
            sreen_raws: win_size.1,
            row_offset: 0,
            column_size: 0,
            render_x: 0,
        }
    }
    fn scroll(&mut self, edit_rows: &EditRows) {
        self.render_x = 0;
        if self.cursor_x < edit_rows.num_of_rows() {
            self.render_x = self.get_render_x(edit_rows.get_editer_row(self.cursor_x))
        }

        self.row_offset = cmp::min(self.row_offset, self.cursor_y);
        if self.cursor_y >= self.row_offset + self.sreen_raws {
            self.row_offset = self.cursor_y - self.sreen_raws + 1;
        }
        self.column_size = cmp::min(self.column_size, self.render_x);
        if self.render_x >= self.column_size + self.screen_columns {
            self.column_size = self.render_x - self.screen_columns + 1;
        }
    }
    fn move_cursor(&mut self, direction: KeyCode, edit_rows: &EditRows) {
        let num_of_rows = edit_rows.num_of_rows();
        match direction {
            KeyCode::Up => {
                self.cursor_y = self.cursor_y.saturating_sub(1);
            }
            KeyCode::Left => {
                if self.cursor_x != 0 {
                    self.cursor_x -= 1;
                } else if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                    self.cursor_x = edit_rows.get_row(self.cursor_y).len();
                }
            }
            KeyCode::Down => {
                if self.cursor_y < num_of_rows {
                    self.cursor_y += 1;
                }
            }
            KeyCode::Right => {
                // if self.cursor_x != self.screen_columns - 1 {
                if self.cursor_y < num_of_rows
                // && self.cursor_x < edit_rows.get_row(self.cursor_y).len()
                {
                    match self.cursor_x.cmp(&edit_rows.get_row(self.cursor_y).len()) {
                        Ordering::Less => self.cursor_x += 1,
                        Ordering::Equal => {
                            self.cursor_y += 1;
                            self.cursor_x = 0;
                        }
                        _ => {}
                    }
                }

                // }
            }
            KeyCode::End => {
                if self.cursor_x < num_of_rows {
                    self.cursor_x = edit_rows.get_row(self.cursor_y).len();
                }
            }
            KeyCode::Home => self.cursor_x = 0,
            _ => unimplemented!(),
        }
        let row_len = if self.cursor_y < num_of_rows {
            edit_rows.get_row(self.cursor_y).len()
        } else {
            0
        };
        self.cursor_x = cmp::min(self.cursor_x, row_len);
    }
    fn get_render_x(&self, row: &Row) -> usize {
        row.row_content[..self.cursor_x]
            .chars()
            .fold(0, |render_x, c| {
                if c == '\t' {
                    render_x + (TAB_STOP - 1) - (render_x % TAB_STOP) + 1
                } else {
                    render_x + 1
                }
            })
    }
}

syntax_struct! {
    struct RustHighlight{
        extensions:["rs"],
        file_type:"rust",
        comment_start:"//",
        keywords: {
            [Color::Yellow;
                "mod","unsafe","extern","crate","use","type","struct","enum","union","const","static",
                "mut","let","if","else","impl","trait","for","fn","self","Self", "while", "true","false",
                "in","continue","break","loop","match"
            ],
            [Color::Reset; "isize","i8","i16","i32","i64","usize","u8","u16","u32","u64","f32","f64",
                "char","str","bool"
            ]
        },
        mutil_comment:Some(("/*","*/"))
    }

}
struct Output {
    win_size: (usize, usize),
    editor_context: EditContents,
    cursor_controller: CursorController,
    edit_rows: EditRows,
    status_message: StatusMessage,
    dirty: u64,
    search_index: SearchIndex,
    syntax_highlight: Option<Box<dyn SyntaxHighlight>>,
}
impl Output {
    fn select_syntax(extenestion: &str) -> Option<Box<dyn SyntaxHighlight>> {
        let list: Vec<Box<dyn SyntaxHighlight>> = vec![Box::new(RustHighlight::new())];
        list.into_iter()
            .find(|it| it.extensions().contains(&extenestion))
    }
    fn new() -> Self {
        let win_size = terminal::size()
            .map(|(x, y)| (x as usize, y as usize - 2))
            .unwrap();
        let mut syntax_highlight = None;

        Self {
            win_size,
            editor_context: EditContents::new(),
            cursor_controller: CursorController::new(win_size),
            edit_rows: EditRows::new(&mut syntax_highlight),
            status_message: StatusMessage::new(
                "Help:Ctrl-F = Find|Ctrl-S = Save |Ctrl-q=quit".into(),
            ),
            dirty: 0,
            search_index: SearchIndex::new(),
            syntax_highlight,
        }
    }
    fn delete_char(&mut self) {
        if self.cursor_controller.cursor_y == self.edit_rows.num_of_rows() {
            return;
        }
        let row = self
            .edit_rows
            .get_editor_row_mut(self.cursor_controller.cursor_y);
        if self.cursor_controller.cursor_x > 0 {
            row.delete_char(self.cursor_controller.cursor_x - 1);
            self.cursor_controller.cursor_x -= 1;
            // self.dirty += 1;
        } else {
            let previous_row = self.edit_rows.get_row(self.cursor_controller.cursor_y - 1);
            self.cursor_controller.cursor_x = previous_row.len();
            self.edit_rows
                .join_adjacent_row(self.cursor_controller.cursor_y);
            self.cursor_controller.cursor_y -= 1;
        }
        if let Some(it) = self.syntax_highlight.as_ref() {
            it.update_syntax(
                self.cursor_controller.cursor_y,
                &mut self.edit_rows.row_contents,
            );
        }
        self.dirty += 1;
    }
    fn insert_newline(&mut self) {
        if self.cursor_controller.cursor_x == 0 {
            self.edit_rows
                .insert_char(self.cursor_controller.cursor_x, String::new())
        } else {
            let current_new = self
                .edit_rows
                .get_editor_row_mut(self.cursor_controller.cursor_y);
            let new_row_content = current_new.row_content[self.cursor_controller.cursor_x..].into();
            current_new
                .row_content
                .truncate(self.cursor_controller.cursor_x);
            self.edit_rows
                .insert_char(self.cursor_controller.cursor_y + 1, new_row_content);
            if let Some(it) = self.syntax_highlight.as_ref() {
                it.update_syntax(
                    self.cursor_controller.cursor_y,
                    &mut self.edit_rows.row_contents,
                );
                it.update_syntax(
                    self.cursor_controller.cursor_y + 1,
                    &mut self.edit_rows.row_contents,
                )
            }
        }
        self.cursor_controller.cursor_x = 0;
        self.cursor_controller.cursor_y += 1;
        self.dirty += 1;
    }
    fn insert_char(&mut self, ch: char) {
        if self.cursor_controller.cursor_y == self.edit_rows.num_of_rows() {
            self.edit_rows
                .insert_char(self.edit_rows.num_of_rows(), String::new());
            self.dirty += 1;
        }
        self.edit_rows
            .get_editor_row_mut(self.cursor_controller.cursor_y)
            .insert_char(self.cursor_controller.cursor_x, ch);
        if let Some(it) = self.syntax_highlight.as_ref() {
            it.update_syntax(
                self.cursor_controller.cursor_y,
                &mut self.edit_rows.row_contents,
            )
        }
        self.cursor_controller.cursor_x += 1;
        self.dirty += 1;
    }
    fn draw_status_bar(&mut self) {
        self.editor_context
            .push_str(&style::Attribute::Reverse.to_string());
        let info = format!(
            "{}{}--{}lines",
            self.edit_rows
                .filename
                .as_ref()
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("[No name]"),
            if self.dirty > 0 { "(modifyed)" } else { " " },
            self.edit_rows.num_of_rows()
        );
        // (0..self.win_size.0).for_each(|_| self.editor_context.push(' '));
        let info_len = cmp::min(info.len(), self.win_size.0);
        let line_info = format!(
            "{}|{}/{}",
            self.syntax_highlight
                .as_ref()
                .map(|highlight| highlight.file_type())
                .unwrap_or("no ft"),
            self.cursor_controller.cursor_y + 1,
            self.edit_rows.num_of_rows()
        );
        self.editor_context.push_str(&info[..info_len]);
        for i in info_len..self.win_size.0 {
            if self.win_size.0 - i == line_info.len() {
                self.editor_context.push_str(&line_info);
                break;
            } else {
                self.editor_context.push(' ')
            }
        }
        self.editor_context
            .push_str(&style::Attribute::Reset.to_string());
        self.editor_context.push_str("\r\n");
    }
    fn draw_message_bar(&mut self) {
        queue!(
            self.editor_context,
            terminal::Clear(ClearType::UntilNewLine)
        )
        .unwrap();
        if let Some(msg) = self.status_message.message() {
            self.editor_context
                .push_str(&msg[..cmp::min(self.win_size.0, msg.len())]);
        }
    }
    fn clear_screen() -> crossterm::Result<()> {
        execute!(stdout(), terminal::Clear(ClearType::All))?;
        execute!(stdout(), cursor::MoveTo(0, 0))
    }
    fn refresh_screen(&mut self) -> crossterm::Result<()> {
        self.cursor_controller.scroll(&self.edit_rows);
        queue!(
            self.editor_context,
            cursor::Hide,
            // terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;

        // Self::clear_screen()?;
        self.draw_rows();
        self.draw_status_bar();
        self.draw_message_bar();
        let cursor_x = self.cursor_controller.render_x - self.cursor_controller.column_size;
        let cursor_y = self.cursor_controller.cursor_y - self.cursor_controller.row_offset;

        queue!(
            self.editor_context,
            cursor::MoveTo(cursor_x as u16, cursor_y as u16),
            cursor::Show
        )?;
        self.editor_context.flush()
        // execute!(stdout(), cursor::MoveTo(0, 0))
    }
    fn find_callback(output: &mut Output, keyboard: &str, key_code: KeyCode) {
        if let Some((index, highligth)) = output.search_index.previous_heiglight.take() {
            output.edit_rows.get_editor_row_mut(index).hithlight = highligth;
        }

        match key_code {
            KeyCode::Esc | KeyCode::Enter => output.search_index.reset(),
            _ => {
                output.search_index.y_direction = None;
                output.search_index.x_direction = None;
                match key_code {
                    KeyCode::Down => {
                        output.search_index.y_direction = SearchDirection::Forward.into()
                    }
                    KeyCode::Up => {
                        output.search_index.y_direction = SearchDirection::Backward.into()
                    }
                    KeyCode::Left => {
                        output.search_index.x_direction = SearchDirection::Backward.into()
                    }
                    KeyCode::Right => {
                        output.search_index.x_direction = SearchDirection::Forward.into()
                    }
                    _ => {}
                }
                for i in 0..output.edit_rows.num_of_rows() {
                    let row_index = match output.search_index.y_direction.as_ref() {
                        None => {
                            if output.search_index.x_direction.is_none() {
                                output.search_index.y_index = i;
                            }
                            output.search_index.y_index
                        }
                        Some(dir) => {
                            if matches!(dir, SearchDirection::Forward) {
                                // output.search_index.y_index = i;
                                // i
                                output.search_index.y_index + i + 1
                            } else {
                                let res = output.search_index.y_index.saturating_sub(i);
                                if res == 0 {
                                    break;
                                }
                                res - 1
                            }
                        }
                    };
                    if row_index > output.edit_rows.num_of_rows() - 1 {
                        break;
                    }

                    let row = output.edit_rows.get_editor_row_mut(row_index);
                    let index = match output.search_index.x_direction.as_ref() {
                        None => row.render.find(&keyboard),
                        Some(dir) => {
                            let index = if matches!(dir, SearchDirection::Forward) {
                                let start =
                                    cmp::min(row.render.len(), output.search_index.x_index + 1);
                                row.render[start..]
                                    .find(&keyboard)
                                    .map(|index| index + start)
                            } else {
                                row.render[..output.search_index.x_index].rfind(&keyboard)
                            };
                            if index.is_none() {
                                break;
                            }
                            index
                        }
                    };

                    if let Some(index) = index {
                        (index..index + keyboard.len())
                            .for_each(|index| row.hithlight[index] = HighlightType::SearchMatch);
                        output.cursor_controller.cursor_y = row_index;
                        output.search_index.y_index = row_index;
                        output.search_index.x_index = index;
                        output.cursor_controller.cursor_x = row.get_row_content_x(index);
                        output.cursor_controller.row_offset = output.edit_rows.num_of_rows();
                        break;
                    }
                }
            }
        }
    }
    fn find(&mut self) -> io::Result<()> {
        // if let Some(keyword) = prompt!(self, "Search:{}(Esc to cancel)") {
        //     for i in 0..self.edit_rows.num_of_rows() {
        //         let row = self.edit_rows.get_editer_row(i);
        //         if let Some(index) = row.render.find(&keyword) {
        //             self.cursor_controller.cursor_y = i;
        //             self.cursor_controller.cursor_x = row.get_row_content_x(index);
        //             self.cursor_controller.row_offset = self.edit_rows.num_of_rows();
        //             break;
        //         }
        //     }
        // }
        // Ok(())
        let curs = self.cursor_controller;
        if prompt!(
            self,
            "Search: {} Use ESC / Arrows / Enter)",
            callback = Output::find_callback
        )
        .is_none()
        {
            self.cursor_controller = curs;
        };
        Ok(())
    }
    fn draw_rows(&mut self) {
        let sreen_raws = self.win_size.1;
        let screen_columns = self.win_size.0;

        for i in 0..sreen_raws {
            let file_row = i + self.cursor_controller.row_offset;
            if file_row >= self.edit_rows.num_of_rows() {
                if i == sreen_raws / 3 {
                    let mut welcome = format!("textedit----version{}", 1);
                    if welcome.len() > screen_columns {
                        welcome.truncate(screen_columns)
                    }
                    let mut padding = (screen_columns - welcome.len()) / 2;
                    if padding != 0 {
                        self.editor_context.push('~');
                        padding -= 1;
                    }
                    (0..padding).for_each(|_| self.editor_context.push(' '));
                    self.editor_context.push_str(&welcome);
                } else {
                    self.editor_context.push('~');
                }
            } else {
                // let len = cmp::min(self.edit_rows.get_row(file_row).len(), screen_columns);
                // self.editor_context
                //     .push_str(&self.edit_rows.get_row(file_row)[..len])
                // let row = self.edit_rows.get_render(file_row);
                let row = self.edit_rows.get_editer_row(file_row);
                let render = &row.render;
                let column_offset = self.cursor_controller.column_size;
                // let len = if row.len() < column_offset {
                //     0
                // } else {
                //     let len = row.len() - column_offset;
                //     if len < screen_columns {
                //         screen_columns
                //     } else {
                //         len
                //     }
                // };
                let len = cmp::min(render.len().saturating_sub(column_offset), screen_columns);

                let start = if len == 0 { 0 } else { column_offset };
                self.syntax_highlight
                    .as_ref()
                    .map(|syntax_highlight| {
                        syntax_highlight.color_row(
                            &render[start..start + len],
                            &row.hithlight[start..start + len],
                            &mut self.editor_context,
                        )
                    })
                    .unwrap_or_else(|| self.editor_context.push_str(&render[start..start + len]));
                // row[start..start + len].chars().for_each(|c| {
                //     if c.is_digit(10) {
                //         let _ = queue!(self.editor_context, SetForegroundColor(Color::Cyan));
                //         self.editor_context.push(c);
                //         let _ = queue!(self.editor_context, ResetColor);
                //     } else {
                //         self.editor_context.push(c);
                //     }
                // });
                // self.editor_context.push_str(&row[start..start + len]);

                // let row = self.editor_rows.get_row(file_row);
                // let column_offset = self.cursor_controller.column_offset;
                // let len = cmp::min(row.len().saturating_sub(column_offset), screen_columns);
                // let start = if len == 0 { 0 } else { column_offset };
                // self.editor_contents
                //     .push_str(&row[start..start + len])
            }
            queue!(
                self.editor_context,
                terminal::Clear(ClearType::UntilNewLine)
            )
            .unwrap();
            // if i < sreen_raws - 1 {
            self.editor_context.push_str("\r\n");
            // }
            // println!("~\r");
        }
        // stdout().flush();
    }
    fn move_cursor(&mut self, direction: KeyCode) {
        self.cursor_controller
            .move_cursor(direction, &self.edit_rows);
    }
}

struct CleanUp;
impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("clound't not disable raw mode");
        Output::clear_screen().expect("Error");
    }
}
struct Reader;
impl Reader {
    fn read_key(&self) -> crossterm::Result<KeyEvent> {
        loop {
            if event::poll(Duration::from_millis(500))? {
                if let Event::Key(event) = event::read()? {
                    return Ok(event);
                }
            }
        }
    }
}

struct Editor {
    reader: Reader,
    output: Output,
    quit_time: u8,
}
impl Editor {
    fn new() -> Self {
        Self {
            reader: Reader,
            output: Output::new(),
            quit_time: QUIT_TIMES,
        }
    }
    fn proceee_keypress(&mut self) -> crossterm::Result<bool> {
        match self.reader.read_key()? {
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: event::KeyModifiers::CONTROL,
            } => {
                if self.output.dirty > 0 && self.quit_time > 0 {
                    self.output.status_message.set_message(format!(
                        "WARNNING!! file has unsaved change press ctrl-q {} more times to quit",
                        self.quit_time
                    ));
                    self.quit_time -= 1;
                    return Ok(true);
                }
            }
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            } => self.output.insert_newline(),
            KeyEvent {
                // code: KeyCode::Char(val @ ('w' | 'a' | 's' | 'd')),
                code:
                    direction @ (KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Home
                    | KeyCode::End),
                modifiers: KeyModifiers::NONE,
            } => self.output.move_cursor(direction),
            KeyEvent {
                code: val @ (KeyCode::PageUp | KeyCode::PageDown),
                modifiers: KeyModifiers::NONE,
            } => {
                if matches!(val, KeyCode::PageUp) {
                    self.output.cursor_controller.cursor_y =
                        self.output.cursor_controller.row_offset;
                } else {
                    self.output.cursor_controller.cursor_y = cmp::min(
                        self.output.win_size.1 + self.output.cursor_controller.row_offset - 1,
                        self.output.edit_rows.num_of_rows(),
                    );
                }

                (0..self.output.win_size.1).for_each(|_| {
                    self.output.move_cursor(if matches!(val, KeyCode::PageUp) {
                        KeyCode::Up
                    } else {
                        KeyCode::Down
                    });
                })
            }
            KeyEvent {
                code: KeyCode::Char('f'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                self.output.find()?;
            }
            KeyEvent {
                code: KeyCode::Char('s'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                if matches!(self.output.edit_rows.filename, None) {
                    // self.output.edit_rows.filename =
                    //     prompt!(&mut self.output, "Save as :{}").map(|it| it.into());
                    let prompt = prompt!(&mut self.output, "Save as : {} (ESC to cancel)")
                        .map(|it| it.into());
                    if let None = prompt {
                        self.output
                            .status_message
                            .set_message("Save aborted".into());
                        return Ok(true);
                    }
                    prompt
                        .as_ref()
                        .and_then(|path: &PathBuf| path.extension())
                        .and_then(|ext| ext.to_str())
                        .map(|ext| {
                            Output::select_syntax(ext).map(|syntax| {
                                let heighlight = self.output.syntax_highlight.insert(syntax);
                                for i in 0..self.output.edit_rows.num_of_rows() {
                                    heighlight
                                        .update_syntax(i, &mut self.output.edit_rows.row_contents)
                                }
                            })
                        });
                    self.output.edit_rows.filename = prompt
                }
                self.output.edit_rows.save().map(|len| {
                    self.output
                        .status_message
                        .set_message(format!("{} bytes wirtten to disk", len));
                    self.output.dirty = 0;
                })?
            }
            KeyEvent {
                code: key @ (KeyCode::Backspace | KeyCode::Delete),
                modifiers: KeyModifiers::NONE,
            } => {
                if matches!(key, KeyCode::Delete) {
                    self.output.move_cursor(KeyCode::Right)
                }
                self.output.delete_char()
            }
            KeyEvent {
                code: code @ (KeyCode::Char(..) | KeyCode::Tab),
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            } => self.output.insert_char(match code {
                KeyCode::Tab => '\t',
                KeyCode::Char(ch) => ch,
                _ => unreachable!(),
            }),
            _ => {}
        }
        Ok(true)
    }
    fn run(&mut self) -> crossterm::Result<bool> {
        self.output.refresh_screen()?;
        self.proceee_keypress()
    }
}

fn main() -> crossterm::Result<()> {
    let _cleanup = CleanUp;
    // terminal::enable_raw_mode().expect("clound't turn on raw mode");
    terminal::enable_raw_mode()?;
    let mut editor = Editor::new();
    while editor.run()? {}
    // loop {
    //     if event::poll(Duration::from_millis(1000)).expect("Error") {
    //         if let Event::Key(event) = event::read().expect("failed to read") {
    //             match event {
    //                 KeyEvent {
    //                     code: KeyCode::Char('q'),
    //                     modifiers: event::KeyModifiers::CONTROL,
    //                 } => break,
    //                 _ => {}
    //             }
    //             println!("{:?}\r", event);
    //         };
    //     } else {
    //         println!("No input yet\r");
    //     }
    // }
    Ok(())
    // let mut buf = [0; 1];
    // while io::stdin().read(&mut buf).expect("failed to read") == 1 && buf != [b'q'] {
    //     let charter = buf[0] as char;
    //     if charter.is_control() {
    //         println!("{}", charter as u8);
    //     } else {
    //         println!("{}", charter);
    //     }
    // }
    // panic!();
}

#[macro_export]
macro_rules! prompt {
    ($output:expr,$args:tt) => {
        prompt!($output, $args, callback = |&_, _, _| {})
    };
    ($output:expr,$args:tt,callback=$callback:expr) => {{
        let output: &mut Output = $output;
        let mut input = String::with_capacity(32);
        loop {
            output.status_message.set_message(format!($args, input));
            output.refresh_screen()?;
            let key_event = Reader.read_key()?;
            match key_event {
                KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::NONE,
                } => {
                    if !input.is_empty() {
                        output.status_message.set_message(String::new());
                        $callback(output, &input, KeyCode::Enter);
                        break;
                    }
                }
                KeyEvent {
                    code: KeyCode::Esc,
                    modifiers: KeyModifiers::NONE,
                } => {
                    output.status_message.set_message(String::new());
                    input.clear();
                    $callback(output, &input, KeyCode::Esc);
                    break;
                }
                KeyEvent {
                    code: KeyCode::Backspace | KeyCode::Delete,
                    modifiers: KeyModifiers::NONE,
                } => {
                    input.pop();
                }
                KeyEvent {
                    code: code @ (KeyCode::Char(..) | KeyCode::Tab),
                    modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                } => input.push(match code {
                    KeyCode::Tab => '\t',
                    KeyCode::Char(ch) => ch,
                    _ => unreachable!(),
                }),
                _ => {}
            }
            $callback(output, &input, key_event.code);
        }
        if input.is_empty() {
            None
        } else {
            Some(input)
        }
    }};
}

#[macro_export]
macro_rules! syntax_struct {
    (struct $Name:ident {
        extensions:$ext:expr,
        file_type:$type:expr,
        comment_start:$start:expr,
        keywords: {
            $([$color:expr; $($words:expr),*]),*
        },
        mutil_comment:$ml_comment:expr

    }
) => {
        struct $Name {
            extensions: &'static [&'static str],
            file_type: &'static str,
            comment_start: &'static str,
            mutil_comment:Option<(&'static str,&'static str)>
        }

        impl $Name {
            fn new() -> Self {
                $(let color=$color;let keyboards=vec!($($words),*);)*

                Self {
                    extensions: &$ext,
                    file_type: $type,
                    comment_start: $start,
                    mutil_comment:$ml_comment,
                }
            }

        }
        impl SyntaxHighlight for $Name {
            fn mutil_comment(&self)->Option<(&str,&str)>{
                self.mutil_comment
            }
            fn comment_start(&self) -> &str {
                self.comment_start
            }
            fn extensions(&self) -> &[&str] {
                self.extensions
            }
            fn file_type(&self) -> &str {
                self.file_type
            }
            fn syntax_color(&self, highlight_type: &HighlightType) -> Color {
                match highlight_type {
                    HighlightType::Normal => Color::Reset,
                    HighlightType::Number => Color::Cyan,
                    HighlightType::SearchMatch => Color::Blue,
                    HighlightType::String => Color::Green,
                    HighlightType::CharLiteral => Color::DarkGreen,
                    HighlightType::Comment|HighlightType::MutilComment => Color::DarkGrey,
                    HighlightType::Other(color)=>*color,
                }
            }
            fn update_syntax(&self, at: usize, edit_rows: &mut Vec<Row>) {
                let mut in_comment=at>0&&edit_rows[at-1].is_comment;
                let current_row = &mut edit_rows[at];
                macro_rules! add {
                    ($h:expr) => {
                        current_row.hithlight.push($h)
                    };
                }
                current_row.hithlight = Vec::with_capacity(current_row.render.len());
                // let chars = current_row.render.chars();
                let render = current_row.render.as_bytes();
                let mut i = 0;
                let mut previous_separator = true;
                let mut in_string: Option<char> = None;
                let comment_start = self.comment_start().as_bytes();
                let mut in_comment=false;
                // for c in chars {
                //     if c.is_digit(10) {
                //         add!(HighlightType::Number);
                //     } else {
                //         add!(HighlightType::Normal);
                //     }
                // }
                while i < render.len() {
                    let c = render[i] as char;
                    let previous_heiglight = if i > 0 {
                        current_row.hithlight[i - 1]
                    } else {
                        HighlightType::Normal
                    };
                    if in_string.is_none() && !comment_start.is_empty()&&!in_comment {
                        let end = i + comment_start.len();
                        if render[i..cmp::min(end, render.len())] == *comment_start {
                            (i..render.len()).for_each(|_| add!(HighlightType::Comment));
                            break;
                        }
                    }
                    if let Some(val)=$ml_comment{
                        if in_string.is_none(){
                            if in_comment{
                                add!(HighlightType::MutilComment);
                                let end=i+val.1.len();
                                if render[i..cmp::min(render.len(),end)]==*val.1.as_bytes(){
                                    (0..val.1.len().saturating_sub(1)).for_each(|_|add!(HighlightType::MutilComment));
                                    i+=val.1.len();
                                    previous_separator=true;
                                    in_comment=false;
                                    continue
                                }else {
                                    i+=1;
                                    continue



                            }


                        }else{
                            let end=i+val.0.len();
                            if render[i..cmp::min(render.len(),end)]==*val.0.as_bytes(){
                                (i..end).for_each(|_|add!(HighlightType::MutilComment));
                                i+=val.0.len();
                                in_comment=true;
                                continue
                            }
                        }
                    }
                }
                    if let Some(val) = in_string {
                        add! {
                            if val=='"'{HighlightType::String}else{HighlightType::CharLiteral}
                        }
                        if c == '\\' && i + 1 < render.len() {
                            add! {
                                if val=='"'{HighlightType::String}else{HighlightType::CharLiteral}
                            }
                            i += 2;
                            continue;
                        }
                        if val == c {
                            in_string = None;
                        }
                        i += 1;
                        previous_separator = true;
                        continue;
                    } else if c == '"' || c == '\'' {
                        in_string = Some(c);
                        add! {
                            if c=='"'{HighlightType::String}else{HighlightType::CharLiteral}
                        }
                        i += 1;
                        continue;
                    }
                    if c.is_digit(10)
                        && (previous_separator
                            || matches!(previous_heiglight, HighlightType::Number))
                        || (c == '.' && matches!(previous_heiglight, HighlightType::Number))
                    {
                        add!(HighlightType::Number);
                        i += 1;
                        previous_separator = false;
                        continue;
                    }
                    if previous_separator {
                        $(
                            $(
                                let end = i + $words.len();
                                let is_end_or_sep = render
                                    .get(end)
                                    .map(|c| self.is_separator(*c as char))
                                    .unwrap_or(end == render.len());
                                if is_end_or_sep && render[i..end] == *$words.as_bytes() {
                                    (i..i + $words.len()).for_each(|_| add!(HighlightType::Other($color)));
                                    i += $words.len();
                                    previous_separator = false;
                                    continue;
                                }
                            )*
                        )*
                    }
                    add!(HighlightType::Normal);
                    previous_separator = self.is_separator(c);
                    i += 1;
                }
                assert_eq!(current_row.render.len(), current_row.hithlight.len());
                let changed=current_row.is_comment!=in_comment;
                current_row.is_comment=in_comment;
                if (changed&&at+1<edit_rows.len()){
                    self.update_syntax(at+1,edit_rows)
                }

            }
        }
    };
}

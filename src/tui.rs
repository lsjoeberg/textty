use crate::page;
use crate::texttv;
use chrono::{DateTime, Local};
use color_eyre::Result;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Flex, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Padding, Paragraph, Widget},
    DefaultTerminal, Frame,
};
use std::borrow::Cow;

impl From<page::Span> for Span<'_> {
    fn from(value: page::Span) -> Self {
        let mut style = Style::default();
        style.bg = Some(value.style.bg.into());
        style.fg = Some(value.style.fg.into());
        Self {
            style,
            content: Cow::from(value.content),
        }
    }
}

impl From<page::BgColour> for ratatui::style::Color {
    fn from(bg: page::BgColour) -> Self {
        match bg {
            page::BgColour::Black => Self::Black,
            page::BgColour::Blue => Self::Blue,
            page::BgColour::Cyan => Self::Cyan,
            page::BgColour::Green => Self::Green,
            page::BgColour::Magenta => Self::Magenta,
            page::BgColour::Red => Self::Red,
            page::BgColour::White => Self::White,
            page::BgColour::Yellow => Self::Yellow,
        }
    }
}

impl From<page::FgColour> for ratatui::style::Color {
    fn from(fg: page::FgColour) -> Self {
        match fg {
            page::FgColour::Black => Self::Black,
            page::FgColour::Blue => Self::Blue,
            page::FgColour::Cyan => Self::Cyan,
            page::FgColour::Green => Self::Green,
            page::FgColour::Magenta => Self::Magenta,
            page::FgColour::Red => Self::Red,
            page::FgColour::White => Self::White,
            page::FgColour::Yellow => Self::Yellow,
        }
    }
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App<'a> {
    page_set: Vec<Text<'a>>,
    page_index: usize,
    page_nr: u16,
    next_nr: u16,
    prev_nr: u16,
    updated_unix: i64,
    mode: Mode,
    input_buffer: String,
    exit: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    Input,
    Help,
}

#[derive(Debug, Default)]
pub struct PageLayout {
    header: Rect,
    content: Rect,
    footer: Rect,
}

impl From<Rect> for PageLayout {
    fn from(area: Rect) -> Self {
        let [area] = Layout::horizontal([Constraint::Length(40)])
            .flex(Flex::Center)
            .areas(area);
        let [header, content, footer] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(24),
            Constraint::Length(2),
        ])
        .flex(Flex::Center)
        .areas(area);
        PageLayout {
            header,
            content,
            footer,
        }
    }
}

struct HelpWidget {}

const HELP_TEXT: &str = r"
READING MODE
←, h  previous page
→, l  next page
↑, k  scroll down
↓, j  scroll up
r     refresh page
1-8   jump to page 100-800
?     show help page
q     quit application

INPUT MODE
:     enter page input mode
↵     submit page number
Esc   exit input mode
";

impl Widget for HelpWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [area] = Layout::horizontal([Constraint::Length(40)])
            .flex(Flex::Center)
            .areas(area);
        let [area] = Layout::vertical([Constraint::Length(24)])
            .flex(Flex::Center)
            .areas(area);
        let help = Paragraph::new(HELP_TEXT).left_aligned().block(
            Block::bordered()
                .title(Line::from(" Help ").left_aligned())
                .title(Line::from(" Esc to Close ").right_aligned())
                .padding(Padding::uniform(1)),
        );
        help.render(area, buf);
    }
}

impl App<'_> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            page_nr: page::HOME_PAGE_NR,
            ..Default::default()
        }
    }

    /// Fetches the current page from the web, parses the page set into
    /// [`Text`] objects, and updates the app state.
    fn get_current_page(&mut self) -> Result<()> {
        let response = texttv::get_page(self.page_nr)?;
        self.next_nr = response.next_page;
        self.prev_nr = response.prev_page;
        self.page_index = 0;
        self.updated_unix = response.date_updated_unix;

        let mut page_set = Vec::with_capacity(response.content.len());
        for content in &response.content {
            let raw_page = page::parse(content)?;
            let text = Text::from(
                raw_page
                    .into_iter()
                    .map(|line| Line::from(line.into_iter().map(Span::from).collect::<Vec<_>>()))
                    .collect::<Vec<_>>(),
            );
            page_set.push(text);
        }

        self.page_set = page_set;
        Ok(())
    }

    /// Go to next page.
    fn next_page(&mut self) -> Result<()> {
        self.page_nr = self.next_nr;
        self.get_current_page()?;
        Ok(())
    }

    /// Go to previous page.
    fn prev_page(&mut self) -> Result<()> {
        self.page_nr = self.prev_nr;
        self.get_current_page()?;
        Ok(())
    }

    /// Go to previous page in page set.
    const fn scroll_prev(&mut self) {
        if self.page_index > 0 {
            self.page_index -= 1;
        }
    }

    /// Go to next page in page set.
    const fn scroll_next(&mut self) {
        let n_pages = self.page_set.len();
        if n_pages > 1 && self.page_index < n_pages - 1 {
            self.page_index += 1;
        }
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        // Get home page on startup.
        self.get_current_page()?;

        while !self.exit {
            terminal.draw(|frame| self.render_ui(frame))?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    /// Renders the user interface.
    fn render_ui(&self, frame: &mut Frame) {
        match self.mode {
            Mode::Normal | Mode::Input => {
                frame.render_widget(self, frame.area());
            }
            Mode::Help => {
                let hw = HelpWidget {};
                frame.render_widget(hw, frame.area());
            }
        }
    }

    /// Reads the crossterm events and updates the state of [`App`].
    fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key)?,
            // Event::Mouse(_) => {}
            // Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match self.mode {
            Mode::Normal => self.handle_key_event_normal(key.code),
            Mode::Input => self.handle_key_event_input(key.code),
            Mode::Help => {
                self.handle_key_event_help(key.code);
                Ok(())
            }
        }
    }

    /// Handle valid events in normal mode.
    fn handle_key_event_normal(&mut self, code: KeyCode) -> Result<()> {
        match code {
            KeyCode::Right | KeyCode::Char('l') => {
                self.page_nr = self.next_nr;
                self.next_page()
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.page_nr = self.prev_nr;
                self.prev_page()
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_prev();
                Ok(())
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_next();
                Ok(())
            }
            KeyCode::Char(a) if a.is_ascii_digit() => {
                if let Some(i) = a.to_digit(10) {
                    if (1..9).contains(&i) {
                        self.page_nr = u16::try_from(i)? * 100;
                        return self.get_current_page();
                    }
                }
                Ok(())
            }
            KeyCode::Char(':') => {
                self.mode = Mode::Input;
                self.input_buffer.clear();
                Ok(())
            }
            KeyCode::Char('?') => {
                self.mode = Mode::Help;
                Ok(())
            }
            KeyCode::Char('r') => self.get_current_page(),
            KeyCode::Char('q') => {
                self.quit();
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Handle events valid in the input mode.
    fn handle_key_event_input(&mut self, code: KeyCode) -> Result<()> {
        match code {
            KeyCode::Char(a) if a.is_ascii_digit() => {
                // Only allow 3-digit page numbers.
                if self.input_buffer.len() < 3 {
                    self.input_buffer.push(a);
                }
                Ok(())
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                Ok(())
            }
            KeyCode::Enter => {
                let requested_page = self.input_buffer.parse::<u16>()?;
                // Wrap page number to valid range.
                if requested_page < page::MIN_PAGE_NR {
                    self.page_nr = page::MIN_PAGE_NR;
                } else if requested_page > page::MAX_PAGE_NR {
                    self.page_nr = page::MAX_PAGE_NR;
                } else {
                    self.page_nr = requested_page;
                }
                self.input_buffer.clear();
                self.mode = Mode::Normal;
                self.get_current_page()
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Handle events valid in the help mode.
    fn handle_key_event_help(&mut self, code: KeyCode) {
        if code == KeyCode::Esc {
            self.mode = Mode::Normal;
        }
    }

    /// Quit the application.
    const fn quit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &App<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = PageLayout::from(area);

        // Current page number, prev/next page, and page index in page set.
        // 0                 19-21               40
        // M------------099-<-100->-101---------1/3
        // |             |     |     |           |
        // margin      prev  curr  next   index/set

        // In command-mode, display the input buffer instead of current page.
        let current_page_str = match self.mode {
            Mode::Normal => self.page_nr.to_string(),
            Mode::Input => self.input_buffer.clone(),
            Mode::Help => String::new(), // FIXME: Remove.
        };
        let scroll_indicator = format!("{}/{}", self.page_index + 1, self.page_set.len());
        let header = Paragraph::new(format!(
            " {:<12}{:>3} ◀ {:>3} ▶ {:>3}{:>12}",
            "", self.prev_nr, current_page_str, self.next_nr, scroll_indicator,
        ))
        .block(
            Block::new()
                .border_style(Style::default().dim())
                .borders(Borders::BOTTOM),
        );
        header.render(layout.header, buf);

        // The current page content.
        let content = Paragraph::new(self.page_set[self.page_index].clone()).centered();
        content.render(layout.content, buf);

        // Add page updated timestamp as page footer.
        let updated = match DateTime::from_timestamp(self.updated_unix, 0) {
            Some(dt) => dt.with_timezone(&Local).format("%H:%M").to_string(),
            None => "N/A".to_string(),
        };
        let footer = Paragraph::new(format!("Sidan uppdaterad: {updated}"))
            .centered()
            .dim()
            .block(Block::new().borders(Borders::TOP));
        footer.render(layout.footer, buf);
    }
}

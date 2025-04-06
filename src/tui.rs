use crate::page;
use crate::ttv;
use chrono::{DateTime, Local};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::Flex;
use ratatui::prelude::{Constraint, Layout, Rect, Stylize};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::{widgets::Paragraph, DefaultTerminal, Frame};
use std::borrow::Cow;

impl From<page::Span> for Span<'_> {
    fn from(value: page::Span) -> Self {
        let mut style = Style::default();

        // Use no-color for `Black` background.
        // FIXME: This may look strange on non-dark-mode.
        let bg = value.style.bg;
        if bg != page::BgColour::Black {
            style.bg = Some(bg.into());
        }

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
            page::BgColour::Black => Self::Black, // FIXME: Looks better with `Reset`
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

#[derive(Debug, Default, Clone)]
pub enum Mode {
    #[default]
    Normal,
    Input,
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
            Constraint::Length(1),
            Constraint::Length(24),
            Constraint::Length(1),
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
        let response = ttv::get_page(self.page_nr)?;
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
    fn scroll_prev(&mut self) {
        if self.page_index > 0 {
            self.page_index -= 1;
        }
    }

    /// Go to next page in page set.
    fn scroll_next(&mut self) {
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
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    /// Renders the user interface.
    fn render(&mut self, frame: &mut Frame) {
        let layout = PageLayout::from(frame.area());

        // Current page number, prev/next page, and page index in page set.
        // 0                 19-21               40
        // M------------099-<-100->-101---------1/3
        // |             |     |     |           |
        // margin      prev  curr  next   index/set

        // In command-mode, display the input buffer instead of current page.
        let current_page_str = match self.mode {
            Mode::Normal => self.page_nr.to_string(),
            Mode::Input => self.input_buffer.clone(),
        };
        let scroll_indicator = format!("{}/{}", self.page_index + 1, self.page_set.len());
        frame.render_widget(
            Paragraph::new(format!(
                " {:<12}{:>3} ◀ {:>3} ▶ {:>3}{:>12}",
                "", self.prev_nr, current_page_str, self.next_nr, scroll_indicator,
            )),
            layout.header,
        );

        // The current page content.
        frame.render_widget(
            Paragraph::new(self.page_set[self.page_index].clone()).centered(),
            layout.content,
        );

        // Add page updated timestamp as page footer.
        let updated = match DateTime::from_timestamp(self.updated_unix, 0) {
            Some(dt) => dt.with_timezone(&Local).format("%H:%M").to_string(),
            None => "N/A".to_string(),
        };
        frame.render_widget(
            Paragraph::new(format!("Sidan uppdaterad: {updated}"))
                .centered()
                .dim(),
            layout.footer,
        );
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
        }
    }

    /// Handle valid events in normal mode.
    fn handle_key_event_normal(&mut self, code: KeyCode) -> Result<()> {
        match code {
            KeyCode::Right => {
                self.page_nr = self.next_nr;
                self.next_page()
            }
            KeyCode::Left => {
                self.page_nr = self.prev_nr;
                self.prev_page()
            }
            KeyCode::Up => {
                self.scroll_prev();
                Ok(())
            }
            KeyCode::Down => {
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

    /// Quit the application.
    fn quit(&mut self) {
        self.exit = true;
    }
}

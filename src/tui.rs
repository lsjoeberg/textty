use crate::{page, texttv};
use chrono::{DateTime, Local};
use color_eyre::Result;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    crossterm::{
        event::{
            self, Event, KeyCode, KeyEvent, KeyEventKind, KeyboardEnhancementFlags,
            PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
        },
        execute,
        terminal::supports_keyboard_enhancement,
    },
    layout::{Constraint, Flex, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Widget},
};
use std::borrow::Cow;
use std::io::stdout;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

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
    client: texttv::Client,
    page_set: Vec<Paragraph<'a>>,
    page_index: usize,
    page_nr: u16,
    next_nr: u16,
    prev_nr: u16,
    updated_unix: i64,
    mode: Mode,
    input_buffer: String,
    use_plain: bool,
    /// Message shown instead of the page content, e.g. when the page could not be displayed.
    status_msg: Option<String>,
    /// Page fetch running off the UI thread. Dropping this drops the receiver, so a late
    /// response from an older fetch is ignored.
    inflight: Option<Inflight>,
    exit: bool,
}

/// Result of a page fetch started on a background thread.
#[derive(Debug)]
struct Inflight {
    rx: Receiver<Result<texttv::PageResponse, crate::error::Error>>,
}

/// How long to wait for a key while a fetch is in flight, so the loop can redraw when it finishes.
const FETCH_POLL: Duration = Duration::from_millis(100);

/// Pops the keyboard protocol on drop, including when the UI panics.
struct KeyEventKinds {
    enabled: bool,
}

impl KeyEventKinds {
    /// Ask the terminal to tag auto-repeat as [`KeyEventKind::Repeat`] and key-up as
    /// [`KeyEventKind::Release`]. Plain keys (`h`, `l`) need every key reported as an escape
    /// sequence, or a hold of those keys still arrives as a stream of presses.
    fn enable() -> Self {
        let wanted = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
        let enabled = supports_keyboard_enhancement().unwrap_or(false)
            && execute!(stdout(), PushKeyboardEnhancementFlags(wanted)).is_ok();
        Self { enabled }
    }
}

impl Drop for KeyEventKinds {
    fn drop(&mut self) {
        if self.enabled {
            let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
        }
    }
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
        let [area] = Layout::horizontal([Constraint::Length(page::N_COLS)])
            .flex(Flex::Center)
            .areas(area);
        let [header, content, footer] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(page::N_ROWS),
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
↑, k  scroll up
↓, j  scroll down
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
        let [area] = Layout::horizontal([Constraint::Length(page::N_COLS)])
            .flex(Flex::Center)
            .areas(area);
        let [area] = Layout::vertical([Constraint::Length(page::N_ROWS)])
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
    pub fn new(plain: bool) -> Self {
        Self {
            client: texttv::Client::new(plain),
            page_nr: texttv::HOME_PAGE_NR,
            use_plain: plain,
            ..Default::default()
        }
    }

    /// Fetch `self.page_nr` on a background thread. The current page stays on screen.
    ///
    /// Replacing [`Self::inflight`] drops the previous receiver. The old thread runs until
    /// the agent timeout, and its response is discarded.
    fn request_page(&mut self) {
        self.status_msg = if self.page_set.is_empty() {
            Some(format!("Hämtar sida {}…", self.page_nr))
        } else {
            None
        };

        let (tx, rx) = mpsc::channel();
        let client = self.client.clone();
        let page_nr = self.page_nr;
        thread::spawn(move || {
            let _ = tx.send(client.get_page(texttv::PageNumber::from(page_nr)));
        });
        self.inflight = Some(Inflight { rx });
    }

    /// Apply a finished fetch. Parse failures stay in the UI as `status_msg`.
    fn apply_page(&mut self, mut response: texttv::PageResponse) {
        self.page_nr = response.num;
        self.next_nr = response.next_page;
        self.prev_nr = response.prev_page;
        self.page_index = 0;
        self.updated_unix = response.date_updated_unix;

        let use_plain = self.use_plain;
        let built = (|| -> Result<_, crate::error::Error> {
            let mut page_set = Vec::with_capacity(response.content.len());
            let mut status = None;
            if use_plain {
                match response.content_plain.take() {
                    Some(content_plain) => {
                        for content in content_plain {
                            page_set.push(Paragraph::new(content).centered());
                        }
                    }
                    None => {
                        status = Some(format!("Oformaterad text saknas för sida {}", response.num));
                    }
                }
            } else {
                for content in &response.content {
                    let raw_page = page::parse(content)?;
                    let lines = raw_page
                        .into_iter()
                        .map(|line| line.into_iter().map(Span::from).collect::<Line>())
                        .collect::<Vec<_>>();
                    page_set.push(Paragraph::new(lines).centered());
                }
            }
            // The API can return a page without any subpages.
            if page_set.is_empty() && status.is_none() {
                status = Some(format!("Sidan {} saknar innehåll", response.num));
            }
            Ok((page_set, status))
        })();

        match built {
            Ok((page_set, status)) => {
                self.page_set = page_set;
                self.status_msg = status;
            }
            Err(err) => {
                self.status_msg = Some(format!("Kunde inte läsa sida {}: {err}", response.num));
            }
        }
    }

    /// Take a finished fetch, if one is ready.
    fn poll_fetch(&mut self) {
        let Some(inflight) = self.inflight.take() else {
            return;
        };
        match inflight.rx.try_recv() {
            Err(TryRecvError::Empty) => self.inflight = Some(inflight),
            Err(TryRecvError::Disconnected) => {
                self.status_msg = Some(format!("Kunde inte hämta sida {}", self.page_nr));
            }
            Ok(Ok(response)) => self.apply_page(response),
            Ok(Err(err)) => {
                self.status_msg = Some(format!("Kunde inte hämta sida {}: {err}", self.page_nr));
            }
        }
    }

    /// Go to next page.
    fn next_page(&mut self) {
        self.page_nr = self.next_nr;
        self.request_page();
    }

    /// Go to previous page.
    fn prev_page(&mut self) {
        self.page_nr = self.prev_nr;
        self.request_page();
    }

    /// Go to previous page in the page set.
    const fn scroll_prev(&mut self) {
        if self.page_index > 0 {
            self.page_index -= 1;
        }
    }

    /// Go to next page in the page set.
    const fn scroll_next(&mut self) {
        let n_pages = self.page_set.len();
        if n_pages > 1 && self.page_index < n_pages - 1 {
            self.page_index += 1;
        }
    }

    /// Run the application's main loop.
    ///
    /// # Errors
    ///
    /// Will return any `Err` that may occur in the application.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let _key_events = KeyEventKinds::enable();
        self.request_page();

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
        // Block while idle. While a fetch is running, wake often enough to draw the result
        // without waiting for the next key.
        let has_event = if self.inflight.is_some() {
            event::poll(FETCH_POLL)?
        } else {
            true
        };
        if has_event {
            match event::read()? {
                // Release is the key going up. Repeat is auto-repeat while it is held.
                Event::Key(key) if key.kind != KeyEventKind::Release => self.on_key_event(key)?,
                // Event::Mouse(_) => {}
                // Event::Resize(_, _) => {}
                _ => {}
            }
        }
        self.poll_fetch();
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match self.mode {
            Mode::Normal => self.handle_key_event_normal(key),
            Mode::Input => self.handle_key_event_input(key.code),
            Mode::Help => {
                self.handle_key_event_help(key.code);
                Ok(())
            }
        }
    }

    /// Handle valid events in normal mode.
    fn handle_key_event_normal(&mut self, key: KeyEvent) -> Result<()> {
        // A held page-nav key is one fetch. Up/down still repeat, because they scroll a page
        // that is already loaded. A press after a release is a new `Press`, so rapid taps count.
        if key.kind == KeyEventKind::Repeat {
            return match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.scroll_prev();
                    Ok(())
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll_next();
                    Ok(())
                }
                _ => Ok(()),
            };
        }

        match key.code {
            KeyCode::Right | KeyCode::Char('l') => {
                self.next_page();
                Ok(())
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.prev_page();
                Ok(())
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
                        self.request_page();
                        return Ok(());
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
            KeyCode::Char('r') => {
                self.request_page();
                Ok(())
            }
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
            // Stay in input mode on empty submit, there's nothing to go to.
            KeyCode::Enter if !self.input_buffer.is_empty() => {
                let requested_page = self.input_buffer.parse::<u16>()?;
                // Wrap page number to valid range.
                self.page_nr = requested_page.clamp(texttv::MIN_PAGE_NR, texttv::MAX_PAGE_NR);
                self.input_buffer.clear();
                self.mode = Mode::Normal;
                self.request_page();
                Ok(())
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

        // Current page number, prev/next page, and page index in the page set.
        // 0                 19-21               40
        // M------------099-<-100->-101---------1/3
        // |             |     |     |           |
        // margin      prev  curr  next   index/set

        // In command-mode, display the input buffer instead of current page.
        let current_page_str = match self.mode {
            Mode::Normal => Cow::Owned(self.page_nr.to_string()),
            Mode::Input => Cow::Borrowed(self.input_buffer.as_str()),
            Mode::Help => Cow::Borrowed(""),
        };
        // Omit the indicator when there are no subpages to count.
        let scroll_indicator = if self.page_set.is_empty() {
            String::new()
        } else {
            format!("{}/{}", self.page_index + 1, self.page_set.len())
        };
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

        // The current page content, or a status message if there is nothing to show.
        // Pages are built once per fetch, so this borrows instead of cloning the page each frame.
        if let Some(status) = &self.status_msg {
            Paragraph::new(status.as_str())
                .centered()
                .render(layout.content, buf);
        } else if let Some(page) = self.page_set.get(self.page_index) {
            page.render(layout.content, buf);
        }

        // Add page updated timestamp as page footer.
        let updated = match DateTime::from_timestamp(self.updated_unix, 0) {
            Some(dt) => dt.with_timezone(&Local).format("%H:%M").to_string(),
            None => "N/A".to_string(),
        };
        let footer_text = if self.inflight.is_some() {
            "Hämtar…".to_string()
        } else {
            format!("Sidan uppdaterad: {updated}")
        };
        let footer = Paragraph::new(footer_text)
            .centered()
            .dim()
            .block(Block::new().borders(Borders::TOP));
        footer.render(layout.footer, buf);
    }
}

use crate::page;
use crate::ttv;
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::prelude::Stylize;
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Block;
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
    page: Text<'a>,
    page_nr: u16,
    next_nr: u16,
    prev_nr: u16,
    exit: bool,
}

impl App<'_> {
    pub fn new() -> Self {
        Self {
            page_nr: 100,
            next_nr: 101,
            prev_nr: 100,
            ..Default::default()
        }
    }

    fn get_current_page(&mut self) -> Result<()> {
        let response = ttv::get_page(self.page_nr)?;
        self.next_nr = response.next_page;
        self.prev_nr = response.prev_page;

        let raw_page = page::parse(response.content.first().unwrap())?;

        let text = Text::from(
            raw_page
                .into_iter()
                .map(|line| Line::from(line.into_iter().map(Span::from).collect::<Vec<_>>()))
                .collect::<Vec<_>>(),
        );
        self.page = text;
        Ok(())
    }

    fn next_page(&mut self) -> Result<()> {
        self.page_nr = self.next_nr;
        self.get_current_page()?;
        Ok(())
    }

    fn prev_page(&mut self) -> Result<()> {
        self.page_nr = self.prev_nr;
        self.get_current_page()?;
        Ok(())
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
    ///
    /// This is where you add new widgets. See the following resources for more information:
    ///
    /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
    /// - <https://github.com/ratatui/ratatui/tree/main/ratatui-widgets/examples>
    fn render(&mut self, frame: &mut Frame) {
        let title = Line::from("textty").bold().blue().centered();
        frame.render_widget(
            Paragraph::new(self.page.clone())
                .centered()
                .block(Block::bordered().title(title)),
            // Rect::new(0, 0, 42, 26).clamp(frame.area()),
            frame.area(),
        );
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
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
        match key.code {
            KeyCode::Right => {
                self.page_nr = self.next_nr;
                self.next_page()
            }
            KeyCode::Left => {
                self.page_nr = self.prev_nr;
                self.prev_page()
            }
            KeyCode::Char('q') => {
                self.quit();
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

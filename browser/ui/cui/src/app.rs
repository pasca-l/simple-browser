use alloc::rc::Rc;
use browser_core::{
    browser::Browser,
    display_item::DisplayItem,
    error::Error,
    http::HttpResponse,
    renderer::layout::computed_style::{FontSize, TextDecoration},
};
use core::cell::RefCell;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::io;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug)]
enum InputMode {
    Normal,
    Editing,
}

#[derive(Clone, Debug, PartialEq)]
struct Link {
    text: String,
    destination: String,
}

impl Link {
    fn new(text: String, destination: String) -> Self {
        Self { text, destination }
    }
}

#[derive(Clone, Debug)]
pub struct Tui {
    browser: Rc<RefCell<Browser>>,
    input_url: String,
    input_mode: InputMode,
    focus: Option<Link>,
}

impl Tui {
    pub fn new(browser: Rc<RefCell<Browser>>) -> Self {
        Self {
            browser,
            input_url: String::new(),
            input_mode: InputMode::Normal,
            focus: None,
        }
    }

    pub fn start(
        &mut self,
        handle_url: fn(String) -> Result<HttpResponse, Error>,
    ) -> Result<(), Error> {
        match enable_raw_mode() {
            Ok(_) => {}
            Err(e) => return Err(Error::Other(format!("{:?}", e))),
        }

        let mut stdout = io::stdout();
        match execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
            Ok(_) => {}
            Err(e) => return Err(Error::Other(format!("{:?}", e))),
        }
        match execute!(stdout, Clear(ClearType::All)) {
            Ok(_) => {}
            Err(e) => return Err(Error::Other(format!("{:?}", e))),
        }
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = match Terminal::new(backend) {
            Ok(t) => t,
            Err(e) => return Err(Error::Other(format!("{:?}", e))),
        };
        match size() {
            Ok(_) => {}
            Err(e) => return Err(Error::Other(format!("{:?}", e))),
        };

        let result = self.run_app(handle_url, &mut terminal);

        match disable_raw_mode() {
            Ok(_) => {}
            Err(e) => return Err(Error::Other(format!("{:?}", e))),
        }
        match execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        ) {
            Ok(_) => {}
            Err(e) => return Err(Error::Other(format!("{:?}", e))),
        }
        match terminal.show_cursor() {
            Ok(_) => {}
            Err(e) => return Err(Error::Other(format!("{:?}", e))),
        }

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::Other(format!("{:?}", e))),
        }
    }

    pub fn browser(&self) -> Rc<RefCell<Browser>> {
        self.browser.clone()
    }

    fn move_focus_up(&mut self) {
        let display_items = self
            .browser
            .borrow()
            .current_page()
            .borrow()
            .display_items();

        // store all links
        let mut focusable_links = Vec::new();
        for item in display_items {
            if let DisplayItem::Text {
                text,
                style,
                layout_point: _,
            } = item
            {
                if style.text_decoration() != TextDecoration::Underline {
                    continue;
                }
                // TODO: get correct destination link from Node.
                let destination = "http://example.com".to_string();
                focusable_links.push(Link::new(text, destination));
            }
        }

        // if focus is not set, on down arrow, focus on the first link
        if self.focus.is_none() {
            if let Some(first_link) = focusable_links.first() {
                self.focus = Some(first_link.clone());
            }
            return;
        }

        // if focus is already set, move to the previous link
        if let Some(current_focus) = &self.focus {
            let current_index = focusable_links
                .iter()
                .position(|link| link == current_focus);

            // loop through the links, in reverse order
            if let Some(index) = current_index {
                let prev_index = if index == 0 {
                    focusable_links.len() - 1
                } else {
                    index - 1
                };
                self.focus = Some(focusable_links[prev_index].clone());
            }
        }
    }

    fn move_focus_down(&mut self) {
        let display_items = self
            .browser
            .borrow()
            .current_page()
            .borrow()
            .display_items();

        // store all links in a vector
        let mut focusable_links = Vec::new();
        for item in display_items {
            if let DisplayItem::Text {
                text,
                style,
                layout_point: _,
            } = item
            {
                if style.text_decoration() != TextDecoration::Underline {
                    continue;
                }
                // TODO: get correct destination link from Node.
                let destination = "http://example.com".to_string();
                focusable_links.push(Link::new(text, destination));
            }
        }

        // if focus is not set, on down arrow, focus on the first link
        if self.focus.is_none() {
            if let Some(first_link) = focusable_links.first() {
                self.focus = Some(first_link.clone());
            }
            return;
        }

        // if focus is already set, move to the next link
        if let Some(current_focus) = &self.focus {
            let current_index = focusable_links
                .iter()
                .position(|link| link == current_focus);

            // loop through the links
            if let Some(index) = current_index {
                let next_index = (index + 1) % focusable_links.len();
                self.focus = Some(focusable_links[next_index].clone());
            }
        }
    }

    fn start_navigation(
        &mut self,
        handle_url: fn(String) -> Result<HttpResponse, Error>,
        destination: String,
    ) -> Result<(), Error> {
        match handle_url(destination) {
            Ok(response) => {
                let page = self.browser.borrow().current_page();
                page.borrow_mut().clear_display_items();
                page.borrow_mut().receive_response(response);
            }
            Err(e) => {
                return Err(e);
            }
        }
        Ok(())
    }

    fn run_app<B: Backend>(
        &mut self,
        handle_url: fn(String) -> Result<HttpResponse, Error>,
        terminal: &mut Terminal<B>,
    ) -> Result<(), Error> {
        loop {
            match terminal.draw(|frame| self.ui(frame)) {
                Ok(_) => {}
                Err(e) => return Err(Error::Other(format!("{:?}", e))),
            }

            let event = match event::read() {
                Ok(event) => event,
                Err(e) => return Err(Error::Other(format!("{:?}", e))),
            };

            match event {
                Event::Key(key) => match self.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Up => {
                            self.move_focus_up();
                        }
                        KeyCode::Down => {
                            self.move_focus_down();
                        }
                        KeyCode::Enter => {
                            if self.focus.is_none() {
                                continue;
                            }

                            if let Some(focus_item) = &self.focus {
                                self.start_navigation(handle_url, focus_item.destination.clone())?;
                            }
                        }
                        KeyCode::Char('e') => {
                            self.input_mode = InputMode::Editing;
                        }
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => {
                            if self.input_url.len() == 0 {
                                continue;
                            }

                            let url: String = self.input_url.drain(..).collect();
                            self.start_navigation(handle_url, url.clone())?;
                        }
                        KeyCode::Char(c) => {
                            self.input_url.push(c);
                        }
                        KeyCode::Backspace => {
                            self.input_url.pop();
                        }
                        KeyCode::Esc => {
                            self.input_mode = InputMode::Normal;
                        }
                        _ => {}
                    },
                },
                Event::Mouse(_) => {
                    // no support for mouse event in Tui browser
                }
                _ => {}
            }
        }
    }

    fn ui(&mut self, frame: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage(5),
                    Constraint::Percentage(25),
                    Constraint::Percentage(50),
                    Constraint::Percentage(20),
                ]
                .as_ref(),
            )
            .split(frame.area());

        let (msg, style) = match self.input_mode {
            InputMode::Normal => (
                vec![
                    Span::raw("Press "),
                    Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to start editing URL, "),
                    Span::styled(
                        "↑ (up arrow)",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" and "),
                    Span::styled(
                        "↓ (down arrow)",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" to move between focused links, "),
                    Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to exit."),
                ],
                Style::default(),
            ),
            InputMode::Editing => (
                vec![
                    Span::raw("Press "),
                    Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to stop editing, "),
                    Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to navigate to the input link."),
                ],
                Style::default(),
            ),
        };
        let text = Text::from(Line::from(msg)).style(style);
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, chunks[0]);

        let input = Paragraph::new(self.input_url.as_str())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::default().borders(Borders::ALL).title("URL"));
        frame.render_widget(input, chunks[1]);

        match self.input_mode {
            InputMode::Normal => {}
            InputMode::Editing => frame.set_cursor_position((
                chunks[1].x + self.input_url.width() as u16 + 1,
                chunks[1].y + 1,
            )),
        }

        let display_items = self
            .browser
            .borrow()
            .current_page()
            .borrow()
            .display_items();

        let mut lines: Vec<Line> = Vec::new();

        for item in display_items {
            match item {
                DisplayItem::Text {
                    text,
                    style,
                    layout_point: _,
                } => {
                    if style.text_decoration() == TextDecoration::Underline {
                        if let Some(focus_item) = &self.focus {
                            if focus_item.text == text {
                                lines.push(Line::from(Span::styled(
                                    text,
                                    Style::default()
                                        .fg(Color::Blue)
                                        .add_modifier(Modifier::UNDERLINED),
                                )));
                                continue;
                            }
                        }
                        lines.push(Line::from(Span::styled(
                            text,
                            Style::default().fg(Color::Blue),
                        )));
                    } else {
                        lines.push(Line::from(if style.font_size() != FontSize::Medium {
                            Span::styled(text, Style::default().add_modifier(Modifier::BOLD))
                        } else {
                            Span::raw(text)
                        }));
                    }
                }
                DisplayItem::Rect { .. } => {}
            }
        }

        let contents = Paragraph::new(Text::from(lines))
            .block(Block::default().title("Content").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(contents, chunks[2]);
    }
}

use browser_core::browser::Browser;
use browser_core::error::Error;
use browser_core::http::HttpResponse;
use iced::widget::{Container, Text};
use iced::{Application, Command, Element, Settings};
use std::cell::RefCell;
use std::rc::Rc;

pub struct Gui {
    _browser: Rc<RefCell<Browser>>,
}

#[derive(Debug, Clone)]
enum Message {}

impl Gui {
    pub fn new(_browser: Rc<RefCell<Browser>>) -> Self {
        Self { _browser }
    }

    pub fn start(
        &mut self,
        _handle_url: fn(String) -> Result<HttpResponse, Error>,
    ) -> Result<(), Error> {
        self.setup()?;

        IcedApp::run(Settings::default()).expect("Failed to run the app");

        Ok(())
    }

    fn setup(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

pub struct IcedApp {}

impl IcedApp {
    pub fn new() -> Self {
        Self {}
    }
}

impl Application for IcedApp {
    type Executor = iced::executor::Default;
    type Message = ();
    type Theme = iced::Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        let app = IcedApp::new();
        (app, Command::none())
    }

    fn title(&self) -> String {
        "Hello, World!".to_string()
    }

    fn update(&mut self, _message: Self::Message) -> Command<Self::Message> {
        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        Container::new(Text::new("Hello, World!"))
            .center_x()
            .center_y()
            .into()
    }
}

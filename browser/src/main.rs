extern crate alloc;

use alloc::rc::Rc;
use browser_core::browser::Browser;
use core::cell::RefCell;

mod handler;

#[cfg(feature = "cui")]
fn create_ui(browser: Rc<RefCell<Browser>>) -> Rc<RefCell<ui_cui::app::Tui>> {
    Rc::new(RefCell::new(ui_cui::app::Tui::new(browser)))
}

#[cfg(feature = "gui")]
fn create_ui(browser: Rc<RefCell<Browser>>) -> Rc<RefCell<ui_gui::app::WasabiUI>> {
    Rc::new(RefCell::new(ui_gui::app::WasabiUI::new(browser)))
}

fn main() {
    let browser = Browser::new();

    let ui = create_ui(browser);

    match ui.borrow_mut().start(handler::handle_url) {
        Ok(_) => {}
        Err(e) => {
            println!("browser fails to start {:?}", e);
        }
    };
}

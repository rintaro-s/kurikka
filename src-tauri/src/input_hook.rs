use parking_lot::Mutex;
use rdev::{listen, Event, EventType};
use std::sync::Arc;

pub struct InputCounter {
    pub clicks: u32,
    pub types: u32,
}

impl InputCounter {
    pub fn new() -> Self {
        Self {
            clicks: 0,
            types: 0,
        }
    }

    pub fn add_click(&mut self) {
        self.clicks += 1;
    }

    pub fn add_type(&mut self) {
        self.types += 1;
    }

    pub fn consume_inputs(&mut self) -> (u32, u32) {
        let clicks = self.clicks;
        let types = self.types;
        self.clicks = 0;
        self.types = 0;
        (clicks, types)
    }
}

pub fn start_input_hook(counter: Arc<Mutex<InputCounter>>) {
    let callback = move |event: Event| match event.event_type {
        EventType::ButtonPress(_) => {
            let mut counter = counter.lock();
            counter.add_click();
        }
        EventType::KeyPress(_) => {
            let mut counter = counter.lock();
            counter.add_type();
        }
        _ => {}
    };

    if let Err(error) = listen(callback) {
        eprintln!("Input hook error: {:?}", error);
    }
}

use std::io::stdout;
use std::io::Stdout;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;

use device_picker::DevicePicker;
use miette::IntoDiagnostic;
use ratatui::crossterm;
use ratatui::crossterm::event;
use ratatui::crossterm::event::DisableBracketedPaste;
use ratatui::crossterm::event::EnableBracketedPaste;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::layout::Rect;
use ratatui::prelude::CrosstermBackend;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Padding;
use ratatui::Frame;
use ratatui::Terminal;
use sane_scan::Sane;
use serde::Deserialize;
use serde::Serialize;

mod device_picker;

enum SaneQuery {
    ListDevices {
        responder: Sender<Vec<sane_scan::Device>>,
    },
}

pub fn tui(sane: Sane) -> miette::Result<()> {
    let (sane_sender, sane_recv) = std::sync::mpsc::channel();
    let mut tui = Tui::new(sane_sender)?;
    crossterm::terminal::enable_raw_mode().into_diagnostic()?;
    crossterm::execute!(stdout(), EnableBracketedPaste).into_diagnostic()?;

    let tui_thread = std::thread::spawn(move || tui.run());

    let sane_handler_res = sane_handler(sane_recv, sane);

    let res = tui_thread.join();

    crossterm::execute!(stdout(), DisableBracketedPaste).into_diagnostic()?;
    crossterm::terminal::disable_raw_mode().into_diagnostic()?;

    match res {
        Ok(res) => sane_handler_res.or(res)?,
        Err(payload) => std::panic::resume_unwind(payload),
    }

    Ok(())
}

fn sane_handler(sane_recv: Receiver<SaneQuery>, sane: Sane) -> miette::Result<()> {
    for query in sane_recv.iter() {
        match query {
            SaneQuery::ListDevices { responder: resp } => {
                let devices = sane.get_devices().into_diagnostic()?;

                if resp.send(devices).is_err() {
                    break;
                }
            }
        }
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct AppConfig {
    active_device: Option<String>,
}

struct App {
    config: AppConfig,
    device_picker: DevicePicker,
}

impl App {
    fn new(sane_sender: Sender<SaneQuery>) -> miette::Result<App> {
        let config = App::load_config()?;
        Ok(App {
            config,
            device_picker: DevicePicker::new(sane_sender.clone()),
        })
    }

    fn load_config() -> miette::Result<AppConfig> {
        Ok(AppConfig::default())
    }

    fn draw(&mut self, frame: &mut Frame) -> miette::Result<()> {
        let outer_block = Block::new()
            .borders(Borders::all())
            .title("scannrs - Scanning made easy")
            .border_type(BorderType::Thick)
            .padding(Padding::uniform(2));

        let rect = outer_block.inner(frame.area());

        frame.render_widget(outer_block, frame.area());

        let Some(selected_device) = self.config.active_device.as_ref() else {
            self.device_picker.draw(frame, rect);
            return Ok(());
        };

        frame.render_widget(selected_device, rect);

        Ok(())
    }

    fn init(&mut self) -> miette::Result<()> {
        self.device_picker.init()?;

        Ok(())
    }

    fn handle_event(&mut self, event: Event) -> miette::Result<Action> {
        if let Event::Key(KeyEvent {
            code: KeyCode::Esc,
            kind: KeyEventKind::Press,
            ..
        }) = event
        {
            return Ok(Action::Quit);
        }

        if self.config.active_device.is_none() {
            let action = self.device_picker.handle_event(Some(event))?;
            if let Some(action) = self.handle_action(action) {
                return Ok(action);
            }
        }

        Ok(Action::Noop)
    }

    fn handle_action(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::SetActiveDevice(device) => self.config.active_device = Some(device),
            _ => return Some(action),
        }

        None
    }
}

struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    app: App,
}

impl Tui {
    fn new(sane_sender: Sender<SaneQuery>) -> miette::Result<Tui> {
        Ok(Tui {
            terminal: Terminal::new(CrosstermBackend::new(stdout())).into_diagnostic()?,
            app: App::new(sane_sender)?,
        })
    }

    fn run(&mut self) -> miette::Result<()> {
        self.app.init()?;
        self.terminal.clear().into_diagnostic()?;
        loop {
            let mut should_break = None;
            self.terminal
                .draw(|frame| {
                    let res = self.app.draw(frame);

                    match res {
                        Ok(()) => {}
                        Err(e) => should_break = Some(Err(e)),
                    }
                })
                .into_diagnostic()?;

            if let Some(res) = should_break {
                break res?;
            }

            if event::poll(Duration::from_millis(100)).into_diagnostic()? {
                let action = match event::read().into_diagnostic()? {
                    event::Event::Key(key) => self.app.handle_event(Event::Key(key))?,
                    event::Event::Resize(w, h) => self.app.handle_event(Event::Resize(w, h))?,
                    _ => Action::Noop,
                };

                match action {
                    Action::Quit => break,
                    _ => (),
                }
            }
        }

        Ok(())
    }
}

enum Action {
    Quit,
    Noop,
    SetActiveDevice(String),
}

enum Event {
    Key(KeyEvent),
    Resize(u16, u16),
    Quit,
}

trait Component {
    fn init(&mut self) -> miette::Result<()> {
        Ok(())
    }

    fn handle_event(&mut self, event: Option<Event>) -> miette::Result<Action> {
        let _ = event;
        Ok(Action::Noop)
    }

    fn draw(&mut self, frame: &mut Frame, rect: Rect);
}

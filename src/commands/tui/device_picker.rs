use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;

use miette::IntoDiagnostic;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::widgets::List;
use ratatui::widgets::ListState;
use sane_scan::Device;

use super::Component;
use super::SaneQuery;

pub struct DevicePicker {
    sane_sender: Sender<SaneQuery>,

    available_devices: Option<Vec<Device>>,
    list_state: ListState,
}
impl DevicePicker {
    pub(crate) fn new(sane_sender: Sender<SaneQuery>) -> Self {
        Self {
            sane_sender,
            available_devices: None,
            list_state: ListState::default(),
        }
    }
}

impl Component for DevicePicker {
    fn init(&mut self) -> miette::Result<()> {
        let (resp, recv) = channel();
        self.sane_sender
            .send(SaneQuery::ListDevices { responder: resp })
            .into_diagnostic()?;

        self.available_devices = Some(recv.recv().into_diagnostic()?);

        self.list_state = ListState::default();

        Ok(())
    }

    fn handle_event(&mut self, event: Option<super::Event>) -> miette::Result<super::Action> {
        match event {
            Some(super::Event::Key(KeyEvent {
                code: KeyCode::Up, ..
            })) => self.list_state.select_previous(),
            Some(super::Event::Key(KeyEvent {
                code: KeyCode::Down,
                ..
            })) => self.list_state.select_next(),

            Some(super::Event::Key(KeyEvent {
                code: KeyCode::Enter,
                ..
            })) => {
                let device = self
                    .available_devices
                    .as_ref()
                    .and_then(|d| d.get(self.list_state.selected()?));

                if let Some(device) = device {
                    return Ok(super::Action::SetActiveDevice(
                        device.name.to_string_lossy().to_string(),
                    ));
                }
            }
            _ => (),
        }

        Ok(super::Action::Noop)
    }

    fn draw(&mut self, frame: &mut ratatui::Frame, rect: ratatui::prelude::Rect) {
        let Some(devices) = self.available_devices.as_ref() else {
            return;
        };

        let [_left, list_area, _right] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(40),
                Constraint::Fill(1),
            ])
            .areas(rect);

        let list = List::new(devices.iter().map(|d| d.name.to_string_lossy()))
            .highlight_style(Style::new().reversed())
            .highlight_symbol(">>");

        frame.render_stateful_widget(list, list_area, &mut self.list_state);
    }
}

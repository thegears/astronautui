use color_eyre::{Result, eyre::Ok};
use crossterm::event::{self, Event};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Rect},
    style::{Style, Stylize},
    text::Text,
    widgets::{Block, List, ListDirection, ListState, Paragraph},
};
use std::process::Command;

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

struct App {
    items: Vec<String>,
    state: ListState,
    info_modal: bool,
    connect_modal: bool,
    connect_modal_input_text: String,
    connect_result_modal: bool,
    connect_result_modal_text: String,
}

impl App {
    fn new(items: Vec<String>) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            items,
            state,
            info_modal: false,
            connect_modal: false,
            connect_modal_input_text: String::new(),
            connect_result_modal: false,
            connect_result_modal_text: String::new(),
        }
    }

    fn up(&mut self) {
        self.state.select_previous();
    }

    fn down(&mut self) {
        self.state.select_next();
    }

    fn update(&mut self) -> Result<()> {
        Command::new("nmcli")
            .args(&["device", "wifi", "rescan"])
            .output()?;
        let networks = Command::new("nmcli")
            .args(&["-t", "-f", "SSID,SIGNAL", "dev", "wifi"])
            .output()?
            .stdout;
        let networks_str = String::from_utf8_lossy(&networks);

        self.items = networks_str
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect();
        Ok(())
    }
}

fn run(mut terminal: DefaultTerminal) -> Result<()> {
    let networks = Command::new("nmcli")
        .args(&["-t", "-f", "SSID,SIGNAL", "dev", "wifi"])
        .output()?
        .stdout;
    let networks_str = String::from_utf8_lossy(&networks);

    let mut app = App::new(
        networks_str
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect(),
    );
    loop {
        terminal.draw(|f| render(f, &mut app))?;
        let key = match event::read()? {
            Event::Key(key) => key,
            _ => continue,
        };

        if app.connect_modal {
            match key.code {
                event::KeyCode::Char(c) => {
                    app.connect_modal_input_text.push(c);
                }
                event::KeyCode::Backspace => {
                    app.connect_modal_input_text.pop();
                }
                event::KeyCode::Enter => {
                    let selected_ssid = app
                        .items
                        .get(app.state.selected().unwrap_or(0))
                        .and_then(|s| s.split(':').next())
                        .unwrap_or("");
                    let password = app.connect_modal_input_text.clone();
                    let connect_result = Command::new("nmcli")
                        .args(&[
                            "dev",
                            "wifi",
                            "connect",
                            selected_ssid,
                            "password",
                            &password,
                            "wifi-sec.key-mgmt",
                            "wpa-psk",
                        ])
                        .output()?
                        .stdout;
                    app.connect_result_modal_text =
                        String::from_utf8_lossy(&connect_result).to_string();
                    app.connect_result_modal = true;
                    app.connect_modal = false;
                    app.connect_modal_input_text.clear();
                }
                event::KeyCode::Esc => {
                    app.connect_modal = false;
                    app.connect_modal_input_text.clear();
                }
                _ => {}
            }
        } else if app.connect_result_modal {
            match key.code {
                event::KeyCode::Enter | event::KeyCode::Esc => {
                    app.connect_result_modal = false;
                    app.connect_result_modal_text.clear();
                }
                _ => {}
            }
        } else {
            match key.code {
                event::KeyCode::Char('q') | event::KeyCode::Esc => break Ok(()),
                event::KeyCode::Char('r') => {
                    terminal.clear()?;
                    app.update()?;
                }

                event::KeyCode::Char('i') => {
                    app.info_modal = !app.info_modal;
                }

                event::KeyCode::Up => {
                    app.up();
                }

                event::KeyCode::Down => {
                    app.down();
                }

                event::KeyCode::Enter => {
                    if app.connect_modal {
                    } else {
                        app.connect_modal = true;
                    }
                }
                _ => continue,
            }
        }
    }
}

fn render(frame: &mut Frame, app: &mut App) {
    let list = List::new(app.items.iter().map(|i| i.as_str()).collect::<Vec<&str>>())
        .block(Block::bordered().title("Astronautui"))
        .style(Style::new().white())
        .highlight_style(Style::new().italic())
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true)
        .direction(ListDirection::TopToBottom);

    let info_modal = Paragraph::new(Text::from(
        "Press 'r' to rescan Wi-Fi networks.
        \nPress 'q' or 'Esc' to quit.
        \nUse Up/Down arrows to navigate.
        \nPress 'i' for this modal.",
    ))
    .block(Block::bordered().title("Instructions"))
    .style(Style::new().yellow())
    .bold()
    .alignment(Alignment::Center);

    if app.info_modal {
        frame.render_widget(
            info_modal,
            Rect::new(
                frame.area().width / 4,
                frame.area().height / 4,
                frame.area().width / 2,
                9,
            ),
        )
    };

    let input_block = Block::bordered()
        .title("Password")
        .border_style(Style::default());

    let input_paragraph =
        Paragraph::new(Text::from(app.connect_modal_input_text.as_str())).block(input_block);

    if app.connect_modal {
        frame.render_widget(
            input_paragraph,
            Rect::new(
                frame.area().width / 4,
                frame.area().height / 4,
                frame.area().width / 2,
                3,
            ),
        )
    };

    let result_paragraph = Paragraph::new(Text::from(app.connect_result_modal_text.as_str()))
        .block(Block::bordered().title("Connection Result"))
        .style(Style::new().green())
        .alignment(Alignment::Center);

    if app.connect_result_modal {
        frame.render_widget(
            result_paragraph,
            Rect::new(
                frame.area().width / 4,
                frame.area().height / 4,
                frame.area().width / 2,
                3,
            ),
        )
    };

    frame.render_stateful_widget(list, frame.area(), &mut app.state);
}

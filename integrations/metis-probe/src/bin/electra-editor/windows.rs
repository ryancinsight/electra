use electra_metis_probe::workbench::Workbench;
use electra_schematic::Schematic;
use metis_core::input::{Accelerator, Key, Modifiers};
use metis_platform::{
    Color, Framebuffer,
    native::{
        MouseButton, NativeApplication, NativeFlow, WindowConfig, WindowEvent,
        run_native_application,
    },
    typeface::{TextSize, TextStyle, draw_text},
};
use std::{error::Error, io, path::PathBuf, time::Duration};

mod plot;

const ROW_TOP: i32 = 72;
struct EditorWindow<'a> {
    app: Workbench<'a>,
    frame: Framebuffer,
    path: PathBuf,
    command: String,
    status: String,
    scroll: usize,
}
impl EditorWindow<'_> {
    fn report(&mut self, result: Result<String, String>) {
        self.status = result.unwrap_or_else(|e| format!("Error: {e}"));
        println!("{}", self.status);
    }
    fn draw(&mut self) {
        self.frame.clear(Color::rgba(21, 28, 39, 255));
        let style = TextStyle::new(Color::WHITE, TextSize::new(15.).expect("positive size"));
        let text = |f: &mut Framebuffer, x, y, s: &str| {
            draw_text(f, x, y, s, style);
        };
        text(&mut self.frame, 16, 12, "Electra   /   Schematic editor");
        text(&mut self.frame, 16, 44, "Sheet / component.pin       Net");
        let count = self.frame.height().saturating_sub(180) as usize / 22;
        let rows = self.app.list().rows();
        if let Some(i) = self
            .app
            .list()
            .selected()
            .and_then(|id| rows.iter().position(|p| *p == id))
        {
            if i < self.scroll {
                self.scroll = i;
            } else if i >= self.scroll + count {
                self.scroll = i.saturating_sub(count.saturating_sub(1));
            }
        }
        for (i, &id) in rows.iter().skip(self.scroll).take(count).enumerate() {
            let net = self
                .app
                .schematic()
                .probe_pin(id)
                .map_or("unwired".into(), |(net, label)| {
                    format!("{}  {}", net.0, label.unwrap_or(""))
                });
            let mark = if self.app.list().selected() == Some(id) {
                ">"
            } else {
                " "
            };
            let sheet = self
                .app
                .schematic()
                .sheet_of(id.instance)
                .unwrap_or("(root)");
            text(
                &mut self.frame,
                16,
                ROW_TOP + 22 * i as i32,
                &format!("{mark} {sheet} / {}.{}     {net}", id.instance, id.pin),
            );
        }
        for (i, line) in [
            "Project commands",
            "place <sheet>   |   save   |   open",
            "wire <net or off>   |   label <name>",
            "bind <V2/R1k/R2k> <pin> <pin>",
            "erc   |   simulate   |   cursor <index>   |   export",
            "bus <name> <nets>   |   lanes <name>",
            "inspect   |   junction <net>",
            "Click or use arrows to select a pin.",
            "Type a command and press Enter. Ctrl+Z to undo.",
        ]
        .iter()
        .enumerate()
        {
            text(&mut self.frame, 380, 44 + i as i32 * 24, line);
        }
        if let Some(curves) = self.app.curves() {
            plot::draw(&mut self.frame, &curves, style);
        }
        let y = self.frame.height() as i32 - 76;
        text(&mut self.frame, 16, y, &self.status);
        text(
            &mut self.frame,
            16,
            y + 34,
            &format!("Command > {}", self.command),
        );
    }
    fn coords(&self, x: i32, y: i32) -> [f64; 2] {
        let bottom = ROW_TOP + ((self.frame.height().saturating_sub(180) / 22) * 22) as i32;
        if !(ROW_TOP..bottom).contains(&y) {
            return [-1., -1.];
        }
        [
            f64::from(x),
            f64::from(y - ROW_TOP) + 22. * self.scroll as f64,
        ]
    }
}
impl NativeApplication for EditorWindow<'_> {
    type Error = io::Error;
    fn framebuffer(&self) -> &Framebuffer {
        &self.frame
    }
    fn handle_events(&mut self, events: &[WindowEvent]) -> Result<NativeFlow, Self::Error> {
        let mut repaint = false;
        for event in events {
            match *event {
                WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                    return Ok(NativeFlow::Exit);
                }
                WindowEvent::Resized { width, height } if width > 0 && height > 0 => {
                    self.frame = Framebuffer::new(width, height).map_err(io::Error::other)?
                }
                WindowEvent::PointerDown {
                    x,
                    y,
                    button: MouseButton::Left,
                } => {
                    self.app
                        .press(self.coords(x, y))
                        .map_err(io::Error::other)?;
                }
                WindowEvent::PointerUp {
                    x,
                    y,
                    button: MouseButton::Left,
                } => {
                    self.app
                        .release(self.coords(x, y))
                        .map_err(io::Error::other)?;
                }
                WindowEvent::FocusLost | WindowEvent::PointerCaptureLost => self.app.cancel(),
                WindowEvent::KeyDown {
                    virtual_key,
                    repeated,
                    modifiers,
                } => {
                    let Some(key) = Key::from_windows_virtual_key(virtual_key) else {
                        continue;
                    };
                    let mods = Modifiers::NONE
                        .with(Modifiers::CTRL, modifiers.ctrl())
                        .with(Modifiers::SHIFT, modifiers.shift())
                        .with(Modifiers::ALT, modifiers.alt());
                    if !repeated && self.app.shortcut(Accelerator::new(mods, key)) {
                        self.report(Ok("history replayed".into()));
                    } else if !mods.is_empty() {
                        continue;
                    } else if key == Key::Enter && !repeated {
                        let line = std::mem::take(&mut self.command);
                        if !line.is_empty() {
                            let r = self.app.execute(&line, &self.path);
                            self.report(r);
                        }
                    } else if key == Key::Backspace {
                        self.command.pop();
                    } else {
                        self.app.navigate(key);
                    }
                }
                WindowEvent::TextInput { character }
                    if !character.is_control() && self.command.len() < 4096 =>
                {
                    self.command.push(character)
                }
                _ => continue,
            }
            repaint = true;
        }
        if repaint {
            self.draw();
        }
        Ok(NativeFlow::Continue { repaint })
    }
}
pub(super) fn run() -> Result<(), Box<dyn Error>> {
    let path = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: electra-editor PROJECT.elcp")?,
    );
    let mut model = Schematic::default();
    let mut window = EditorWindow {
        app: Workbench::new(&mut model),
        path,
        frame: Framebuffer::new(960, 650)?,
        command: String::new(),
        status: "Place a channel or open the saved project.".into(),
        scroll: 0,
    };
    window.draw();
    run_native_application(
        &WindowConfig::new("Electra schematic editor", 960, 650)?,
        window,
        Duration::from_millis(50),
    )?;
    Ok(())
}

//! Command surface shared by native event handling and deterministic project replay.
mod channel;
mod dc;
use crate::{PinEditor, PinList};
pub use dc::DcCurves;
use electra_schematic::{ModelBinding, NetId, PinId, Schematic};
use metis_core::input::{Accelerator, Key};
use std::{
    fs,
    io::{BufReader, BufWriter, Write},
    path::Path,
    sync::Arc,
};

/// Native editor document, selection and most recent selected DC sweep.
pub struct Workbench<'a> {
    editor: PinEditor<'a>,
    list: PinList<'static>,
    trace: Option<dc::Trace>,
}
impl<'a> Workbench<'a> {
    /// Start with a project and list its placed pins.
    pub fn new(model: &'a mut Schematic) -> Self {
        let mut state = Self {
            editor: PinEditor::new(model),
            list: PinList::new(&[], 360., 22.).expect("positive row geometry"),
            trace: None,
        };
        state.refresh();
        state
    }
    fn refresh(&mut self) {
        self.list
            .replace_rows(self.editor.schematic().pins().map(|(id, _)| id).collect());
        self.trace = None;
    }
    /// Visible hit-test rows and keyboard focus.
    pub fn list(&self) -> &PinList<'static> {
        &self.list
    }
    /// Current schematic for presentation.
    pub fn schematic(&self) -> &Schematic {
        self.editor.schematic()
    }
    /// Borrow the solved sweep columns for drawing. The cursor reads the same views.
    pub fn curves(&self) -> Option<DcCurves<'_>> {
        self.trace.as_ref()?.curves().ok()
    }
    /// Forward a primary press in row-list coordinates.
    pub fn press(&mut self, xy: [f64; 2]) -> Result<(), String> {
        self.list.press(xy).map_err(|e| e.to_string())
    }
    /// Resolve a primary release and invalidate a trace if selection changed.
    pub fn release(&mut self, xy: [f64; 2]) -> Result<(), String> {
        let old = self.list.selected();
        self.list.release(xy).map_err(|e| e.to_string())?;
        if old != self.list.selected() {
            self.trace = None;
        }
        Ok(())
    }
    /// Drop pointer capture on focus loss.
    pub fn cancel(&mut self) {
        self.list.cancel_press();
    }
    /// Navigate the visible rows.
    pub fn navigate(&mut self, key: Key) -> bool {
        let before = self.list.selected();
        let handled = self.list.key(key);
        if before != self.list.selected() {
            self.trace = None;
        }
        handled
    }
    /// Replay a Metis accelerator against edit history.
    pub fn shortcut(&mut self, key: Accelerator) -> bool {
        let done = self.editor.shortcut(key);
        if done {
            self.trace = None;
        }
        done
    }
    /// Execute an editor command. Save/open use the caller's project path.
    pub fn execute(&mut self, line: &str, path: &Path) -> Result<String, String> {
        let (verb, arg) = line.trim().split_once(' ').unwrap_or((line.trim(), ""));
        let arg = arg.trim();
        match verb {
            "undo" | "redo" => {
                let chord = if verb == "undo" {
                    "CmdOrCtrl+Z"
                } else {
                    "CmdOrCtrl+Shift+Z"
                };
                if !self.shortcut(Accelerator::parse(chord).expect("valid Metis accelerator")) {
                    return Err("no edit to replay".into());
                }
            }
            "place" => {
                channel::place(&mut self.editor, arg)?;
                self.refresh();
            }
            "wire" => {
                let net = if arg == "off" {
                    None
                } else {
                    Some(NetId(arg.parse().map_err(|_| "enter a net number or off")?))
                };
                self.editor
                    .wire(&self.list, net)
                    .map_err(|e| format!("{e:?}"))?;
                self.trace = None;
            }
            "label" => {
                self.editor
                    .label(&self.list, Some(arg.to_owned()))
                    .map_err(|e| format!("{e:?}"))?;
                self.trace = None;
            }
            "bind" => {
                let mut words = arg.split_whitespace();
                let name = words
                    .next()
                    .ok_or("enter a model and its two ordered pins")?;
                dc::part(name)?;
                let pins = words
                    .map(str::parse)
                    .collect::<Result<Vec<u32>, _>>()
                    .map_err(|_| "invalid pin number")?;
                if pins.len() != 2 {
                    return Err("model needs two pins".into());
                }
                self.editor
                    .bind(
                        &self.list,
                        Some(Arc::new(ModelBinding::new(name.into(), pins)?)),
                    )
                    .map_err(|e| format!("{e:?}"))?;
                self.trace = None;
            }
            "bus" => {
                let mut words = arg.split_whitespace();
                let name = words.next().ok_or("enter a bus name and ordered net IDs")?;
                let lanes = words
                    .map(|word| word.parse().map(NetId))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| "invalid bus lane")?;
                self.editor
                    .set_bus(name.to_owned(), lanes)
                    .map_err(|e| format!("{e:?}"))?;
            }
            "lanes" => {
                let lanes = self.schematic().bus(arg).ok_or("unknown bus")?;
                let mut result = String::from("lanes");
                use std::fmt::Write;
                for lane in lanes {
                    write!(result, " {}", lane.0).expect("write to string");
                }
                return Ok(result);
            }
            "inspect" => {
                let id = self.list.selected().ok_or("select a pin")?;
                let model = self.schematic();
                let sheet = model.sheet_of(id.instance).unwrap_or("(root)");
                let binding = model.model(id.instance);
                return Ok(format!(
                    "sheet {sheet} part {} pin {} model {} {:?}",
                    id.instance,
                    id.pin,
                    binding.map_or("(none)", |b| b.name()),
                    binding.map_or(&[][..], |b| b.pins())
                ));
            }
            "junction" => {
                let net = NetId(arg.parse().map_err(|_| "enter a net number")?);
                let mut result = String::from("pins");
                use std::fmt::Write;
                for id in self.schematic().junction(net) {
                    write!(result, " {}.{}", id.instance, id.pin).expect("write to string");
                }
                return Ok(result);
            }
            "erc" => return Ok(format!("ERC {:?}", dc::erc(self.schematic()))),
            "simulate" => {
                self.trace = None;
                let trace = dc::Trace::solve(
                    self.schematic(),
                    self.list.selected().ok_or("select a pin")?,
                )?;
                let result = trace.cursor(0)?;
                self.trace = Some(trace);
                return Ok(result);
            }
            "cursor" => {
                return self
                    .trace
                    .as_ref()
                    .ok_or("simulate this selection first")?
                    .cursor(arg.parse().map_err(|_| "invalid sample index")?);
            }
            "save" => save(path, |w| self.schematic().write_connectivity(w))?,
            "open" => {
                let file = fs::File::open(path).map_err(|e| e.to_string())?;
                self.editor.open(
                    Schematic::read_connectivity(BufReader::new(file))
                        .map_err(|e| e.to_string())?,
                );
                self.refresh();
            }
            "export" => {
                let deck = dc::deck(self.schematic())?;
                let dest = path.with_extension("cir");
                if dest == path {
                    return Err("deck and project paths collide".into());
                }
                save(&dest, |w| w.write_all(deck.as_bytes()))?;
            }
            _ => return Err("unknown command".into()),
        }
        Ok(format!("{verb} complete"))
    }
}

fn save(
    path: &Path,
    write: impl FnOnce(&mut BufWriter<fs::File>) -> std::io::Result<()>,
) -> Result<(), String> {
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".{}.tmp", std::process::id()));
    let tmp = std::path::PathBuf::from(name);
    let file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&tmp)
        .map_err(|e| e.to_string())?;
    let mut w = BufWriter::new(file);
    let result = write(&mut w)
        .and_then(|()| w.flush())
        .and_then(|()| w.get_ref().sync_all());
    drop(w);
    if let Err(e) = result.and_then(|()| fs::rename(&tmp, path)) {
        let _ = fs::remove_file(tmp);
        return Err(e.to_string());
    }
    Ok(())
}

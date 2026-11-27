//! One reusable supply and resistive divider sheet.
use super::*;
use std::collections::BTreeMap;

pub(super) fn place(editor: &mut PinEditor<'_>, path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("enter a sheet path".into());
    }
    let project = editor.schematic();
    let name = format!("{path}/OUT");
    if project.labels().any(|(_, label)| label == name) {
        return Err("sheet output name is already in use".into());
    }
    let last = project.pins().last().map_or(0, |(id, _)| id.instance);
    let last_net = project
        .pins()
        .filter_map(|(_, p)| p.net)
        .chain(project.labels().map(|(net, _)| net))
        .max()
        .unwrap_or(NetId(0))
        .0;
    last.checked_add(3).ok_or("component IDs exhausted")?;
    last_net.checked_add(2).ok_or("net IDs exhausted")?;
    let mut template = Schematic::default();
    for (part, name, a, b) in [(1, "V2", 1, 0), (2, "R1k", 1, 2), (3, "R1k", 2, 0)] {
        for (pin, net) in [(1, a), (2, b)] {
            template.insert(
                PinId {
                    instance: part,
                    pin,
                },
                electra_schematic::Pin {
                    net: Some(NetId(net)),
                    required: true,
                    role: if part == 1 && pin == 1 {
                        electra_schematic::Role::Output
                    } else {
                        electra_schematic::Role::Passive
                    },
                },
            );
        }
        template.bind_model(part, name.into(), vec![1, 2])?;
    }
    template.set_label(NetId(1), "VIN".into())?;
    template.set_label(NetId(2), "OUT".into())?;
    let parts = (1..=3).map(|i| (i, last + i)).collect();
    let nets = BTreeMap::from([
        (NetId(0), NetId(0)),
        (NetId(1), NetId(last_net + 1)),
        (NetId(2), NetId(last_net + 2)),
    ]);
    editor
        .place(&template, path, &parts, &nets)
        .map_err(|e| format!("{e:?}"))
}

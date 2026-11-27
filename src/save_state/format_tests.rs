//! Checks that the save state format doesn't change unnoticed.
//!
//! A save state of the current format is committed in `fixtures/`, with what
//! it decodes to, written as JSON with the field names. If a saved field is
//! added, removed, renamed, moved or changes type, the committed state no
//! longer decodes, or decodes to other values, and the test fails.
//!
//! When the format changes on purpose:
//! 1. bump [`FORMAT_VERSION`];
//! 2. run `cargo test write_format_fixture -- --ignored`, which writes the
//!    fixture of the new format;
//! 3. point [`FIXTURE_VERSION`], [`FIXTURE`] and [`EXPECTED`] to the new
//!    files. The old ones should stay, to test potential future migration.

use super::test_utils::{MASTER_CYCLES_PER_FRAME, test_rom_with_sram};
use super::{FORMAT_VERSION, SaveState};
use crate::test_utils::{make_rsnes_from, tick_core};
use serde_json::{Value, json};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Format version of the committed save state.
const FIXTURE_VERSION: u32 = 1;

/// A save state of format [`FIXTURE_VERSION`], made by `write_format_fixture`.
const FIXTURE: &[u8] = include_bytes!("fixtures/format-1.rsnes-state");

/// What [`FIXTURE`] decodes to, see [`describe`].
const EXPECTED: &str = include_str!("fixtures/format-1.json");

/// Arrays longer than this are written as their length and CRC32.
const MAX_ARRAY_LENGTH: usize = 32;

/// A decoded save state as JSON, with the field names. The long arrays
/// (memories, framebuffers...) are summed up by their length and CRC32, to
/// keep the expected file small.
fn describe(state: &SaveState) -> Value {
    let mut value = json!({
        "info": serde_json::to_value(&state.info).unwrap(),
        "core": serde_json::to_value(&state.core).unwrap(),
    });
    shorten_arrays(&mut value);
    value
}

/// Replaces the arrays longer than [`MAX_ARRAY_LENGTH`] by their length and
/// the CRC32 of their JSON text.
fn shorten_arrays(value: &mut Value) {
    match value {
        Value::Array(items) if items.len() > MAX_ARRAY_LENGTH => {
            let text = serde_json::to_string(items).unwrap();
            *value = json!({
                "len": items.len(),
                "crc32": format!("{:08X}", crc32fast::hash(text.as_bytes())),
            });
        }
        Value::Array(items) => items.iter_mut().for_each(shorten_arrays),
        Value::Object(fields) => fields.values_mut().for_each(shorten_arrays),
        _ => {}
    }
}

/// Where `found` first differs from `expected`, with the path of the field.
/// Field order counts.
fn first_difference(expected: &Value, found: &Value, path: &str) -> Option<String> {
    match (expected, found) {
        (Value::Object(expected), Value::Object(found)) => {
            if !expected.keys().eq(found.keys()) {
                let names = |fields: &serde_json::Map<String, Value>| {
                    fields.keys().cloned().collect::<Vec<_>>().join(", ")
                };
                return Some(format!(
                    "{path}: expected the fields [{}], found [{}]",
                    names(expected),
                    names(found)
                ));
            }
            expected.iter().find_map(|(name, value)| {
                first_difference(value, &found[name], &format!("{path}.{name}"))
            })
        }
        (Value::Array(expected), Value::Array(found)) if expected.len() == found.len() => expected
            .iter()
            .zip(found)
            .enumerate()
            .find_map(|(index, (expected, found))| {
                first_difference(expected, found, &format!("{path}[{index}]"))
            }),
        _ if expected == found => None,
        _ => Some(format!("{path}: expected {expected}, found {found}")),
    }
}

#[test]
fn the_save_state_format_has_not_changed() {
    assert_eq!(
        FIXTURE_VERSION, FORMAT_VERSION,
        "FORMAT_VERSION changed: write a fixture of the new format, \
         see the doc of save_state::format_tests"
    );
    let state = SaveState::decode(FIXTURE).unwrap_or_else(|error| {
        panic!(
            "the committed save state no longer decodes ({error}). If the format \
             changed on purpose, see the doc of save_state::format_tests"
        )
    });
    let expected: Value = serde_json::from_str(EXPECTED).unwrap();

    if let Some(difference) = first_difference(&expected, &describe(&state), "state") {
        panic!(
            "the committed save state decodes to other values, the format changed: \
             {difference}. If the change is on purpose, see the doc of \
             save_state::format_tests"
        );
    }
}

#[test]
fn a_difference_names_the_field() {
    let expected = json!({ "core": { "a": 1, "b": [1, 2] } });

    assert_eq!(first_difference(&expected, &expected, "state"), None);
    assert_eq!(
        first_difference(
            &expected,
            &json!({ "core": { "a": 1, "b": [1, 3] } }),
            "state"
        ),
        Some("state.core.b[1]: expected 2, found 3".to_owned())
    );
    assert_eq!(
        first_difference(
            &expected,
            &json!({ "core": { "b": [1, 2], "a": 1 } }),
            "state"
        ),
        Some("state.core: expected the fields [a, b], found [b, a]".to_owned())
    );
}

#[test]
fn long_arrays_are_summed_up() {
    let short = vec![0; MAX_ARRAY_LENGTH];
    let long = vec![0; MAX_ARRAY_LENGTH + 1];
    let mut value = json!({ "short": short, "long": long });
    shorten_arrays(&mut value);

    assert_eq!(value["short"], json!(short));
    assert_eq!(value["long"]["len"], MAX_ARRAY_LENGTH + 1);
    assert!(value["long"]["crc32"].is_string());
}

/// Writes the fixture of the current format. Never overwrites one: the
/// fixture of a format must not change.
#[test]
#[ignore = "writes the fixture of a new format, see the module doc"]
fn write_format_fixture() {
    let mut core = make_rsnes_from(&test_rom_with_sram());
    // A few frames and a bit more: the console is in the middle of a frame,
    // and most values are no longer at their reset value.
    tick_core(
        &mut core,
        3 * MASTER_CYCLES_PER_FRAME + MASTER_CYCLES_PER_FRAME / 2,
    );
    let bytes = core.save_state().unwrap();
    let state = SaveState::decode(&bytes).unwrap();
    let json = serde_json::to_string_pretty(&describe(&state)).unwrap() + "\n";

    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/save_state/fixtures");
    let name = format!("format-{FORMAT_VERSION}");
    write_new(&folder.join(format!("{name}.rsnes-state")), &bytes);
    write_new(&folder.join(format!("{name}.json")), json.as_bytes());
}

/// Writes a file that must not exist yet.
fn write_new(path: &Path, bytes: &[u8]) {
    let mut file = File::create_new(path).unwrap_or_else(|error| {
        panic!(
            "{}: {error}. The fixture of a format must not change: \
             bump FORMAT_VERSION first",
            path.display()
        )
    });
    file.write_all(bytes).unwrap();
}

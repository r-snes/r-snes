use std::{path::PathBuf, string::FromUtf8Error};

use piccolo::{Context, Value};
use product_order::combine_ordering;

use super::{FileWritePermissions, PermTreeFromAllOr, PermTreeNode};

#[derive(Default, PartialEq, Eq, PartialOrd, Debug)]
pub struct FileWriteOptions {}

impl PartialOrd for FileWritePermissions {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering::*;

        let mut acc = Equal;

        // first compare against files which are in self, which
        // may or may not be in other.
        // in this loop we are sure to catch all files which are in both
        for (filepath, write_options) in &self.files {
            if let Some(other_options) = other.files.get(filepath) {
                // both have the same file, combine with the comparison of
                // write options
                acc = combine_ordering(acc, write_options.partial_cmp(other_options)?)?;
            } else {
                // self has a file which other doesn't, so combine with greater
                acc = combine_ordering(acc, Greater)?;
            }
        }

        // second loop to catch files which are in other but not in self
        for filepath in other.files.keys() {
            if !self.files.contains_key(filepath) {
                // other has a file which self doesn't, so combine with less
                acc = combine_ordering(acc, Less)?;
            }
        }

        Some(acc)
    }
}

impl PermTreeFromAllOr for FileWritePermissions {
    fn from_lua_inner<'gc>(ctx: Context<'gc>, value: Value<'gc>) -> Option<Self> {
        let Value::Table(tab) = value else {
            eprintln!("write permissions should be a table");
            return None;
        };

        let mut ret = Self::default();

        for (key, val) in tab {
            match (key, val) {
                (Value::Integer(_), Value::String(file)) => {
                    let pathbuf = match picc_string_to_path(file) {
                        Ok(p) => p,
                        Err(e) => {
                            eprintln!("error reading path as utf8: {e}");
                            continue;
                        }
                    };
                    ret.files.insert(pathbuf, Default::default());
                }
                (Value::String(file), v) => {
                    let pathbuf = match picc_string_to_path(file) {
                        Ok(p) => p,
                        Err(e) => {
                            eprintln!("error reading path as utf8: {e}");
                            continue;
                        }
                    };

                    ret.files
                        .insert(pathbuf, FileWriteOptions::from_lua(ctx, v)?);
                }
                _ => eprintln!("unexpected key val combo in file write permissions"),
            }
        }
        Some(ret)
    }
}

impl PermTreeNode for FileWriteOptions {
    fn from_lua<'gc>(_: Context<'gc>, _: Value<'gc>) -> Option<Self> {
        // since it's a unit struct for now there's nothing to do
        Some(Self {})
    }
}

fn picc_string_to_path<'gc>(string: piccolo::String<'gc>) -> Result<PathBuf, FromUtf8Error> {
    let byte_vec = Vec::from(string.as_bytes());
    cfg_select! {
        unix => {{
            use std::ffi::OsString;
            use std::os::unix::ffi::OsStringExt;

            Ok(PathBuf::from(OsString::from_vec(byte_vec)))
        }},
        target_os = "wasi" => {{
            use std::ffi::OsString;
            use std::os::wasi::ffi::OsStringExt;

            Ok(PathBuf::from(OsString::from_vec(byte_vec)))
        }},
        _ => String::from_utf8(byte_vec).map(PathBuf::from),
    }
}

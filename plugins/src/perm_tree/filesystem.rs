use std::{path::PathBuf, string::FromUtf8Error};

use piccolo::{Context, Value};
use product_order::combine_ordering;

use super::{FileWritePermissions, PermTreeFromAllOr, PermTreeNode};

#[derive(Copy, Clone, Eq, Debug)]
pub enum FileWriteOptions {
    /// Only create a new file, don't overwrite
    /// (or even append) an existing file
    NewOnly,

    /// May overwrite (at least append) an existing file
    CanOverwrite { create: bool, mode: OverwriteMode },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum OverwriteMode {
    /// Open in append mode but don't allow seeking at all,
    /// to prevent modification of existing data in the file
    AppendOnly,

    /// Open in append mode, allow seeking
    Append,

    /// Open in truncate mode (will completely overwrite the file)
    /// Seeking is then allowed because all data is erased anyways
    Truncate,

    /// Open the file from that start, then any write will
    /// overwrite data from the start.
    /// Seeking is allowed since we're already overwriting data.
    Start,
}

impl FileWriteOptions {
    /// Whether these file write options will allow
    /// seeking the opened file
    pub fn can_seek(self) -> bool {
        match self {
            Self::CanOverwrite {
                mode: OverwriteMode::AppendOnly,
                ..
            } => false,
            _ => true,
        }
    }

    /// Whether these file write options may create
    /// new files on the user's machine
    pub fn can_create_new(self) -> bool {
        match self {
            Self::NewOnly => true,
            Self::CanOverwrite { create, .. } => create,
        }
    }

    /// Whether these file write options may touch
    /// existing files at all
    pub fn can_touch_existing(self) -> bool {
        !matches!(self, Self::NewOnly)
    }

    /// Whether these file write options may overwrite
    /// existing data in files (just appending counts as false)
    pub fn can_overwrite_existing(self) -> bool {
        use OverwriteMode::*;

        match self {
            Self::NewOnly => false,
            Self::CanOverwrite {
                mode: AppendOnly, ..
            } => false,
            _ => true,
        }
    }
}

impl Default for FileWriteOptions {
    /// Default options to apply when constructed from
    /// just the file name, or with `"file" = "all"`
    fn default() -> Self {
        Self::CanOverwrite {
            create: true,
            mode: OverwriteMode::Truncate,
        }
    }
}

impl PartialEq for FileWriteOptions {
    fn eq(&self, other: &Self) -> bool {
        self.can_create_new() == other.can_create_new()
            && self.can_touch_existing() == other.can_touch_existing()
            && self.can_overwrite_existing() == other.can_overwrite_existing()
    }
}

impl PartialOrd for FileWriteOptions {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        product_order::combine_option_orderings(
            [
                Self::can_create_new,
                Self::can_touch_existing,
                Self::can_overwrite_existing,
            ]
            .map(|cmp| cmp(*self).partial_cmp(&cmp(*other))),
        )
    }
}

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
    fn from_lua<'gc>(ctx: Context<'gc>, value: Value<'gc>) -> Option<Self> {
        use OverwriteMode::*;

        match value {
            Value::String(s) if s.as_bytes() == b"all" => Some(Default::default()),
            Value::String(s) if s.as_bytes() == b"create_only" => Some(Self::NewOnly),
            Value::Table(tab) => {
                let create = match tab.get_value(ctx, "create") {
                    Value::Boolean(b) => b,
                    Value::Nil => false,
                    _ => {
                        eprintln!(
                            "invalid value for 'create' in overwrite options, assuming false"
                        );
                        false
                    }
                };

                let mode = match tab.get_value(ctx, "mode") {
                    Value::String(s) if s.as_bytes() == b"append_only" => AppendOnly,
                    Value::String(s) if s.as_bytes() == b"append" => Append,
                    Value::String(s) if s.as_bytes() == b"truncate" => Truncate,
                    Value::String(s) if s.as_bytes() == b"start" => Start,
                    Value::Nil => {
                        eprintln!("missing value for 'mode' in overwrite options");
                        return None;
                    }
                    _ => {
                        eprintln!("invalid value for 'mode' in overwrite options");
                        return None;
                    }
                };

                Some(Self::CanOverwrite { create, mode })
            }
            _ => {
                eprintln!("invalid value to construct file write opts");
                None
            }
        }
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

impl From<FileWriteOptions> for std::fs::OpenOptions {
    fn from(options: FileWriteOptions) -> Self {
        let mut ret = Self::new();

        ret.read(false);
        ret.write(true);
        ret.create(options.can_create_new());
        match options {
            FileWriteOptions::NewOnly => {
                ret.create_new(true);
            }
            FileWriteOptions::CanOverwrite { mode, .. } => {
                ret.create_new(false);

                match mode {
                    OverwriteMode::AppendOnly | OverwriteMode::Append => {
                        ret.truncate(false);
                        ret.append(true);
                    }
                    OverwriteMode::Truncate => {
                        ret.truncate(true);
                        ret.append(false);
                    }
                    OverwriteMode::Start => {
                        ret.truncate(false);
                        ret.append(false);
                    }
                }
            }
        };

        ret
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;
    use crate::{
        perm_tree::{FileWritePermissions, test::build_from_lua},
        permission::helpers::AllOr,
    };

    fn build_file_write_perms(lua: &str) -> AllOr<FileWritePermissions> {
        build_from_lua(lua, <AllOr<FileWritePermissions> as PermTreeNode>::from_lua)
            .expect("valid construction")
    }

    fn build_file_write_opts(lua: &str) -> FileWriteOptions {
        build_from_lua(lua, FileWriteOptions::from_lua).expect("valid construction")
    }

    #[test]
    fn create_with_no_files() {
        let from_none = build_file_write_perms("\"none\"");
        let from_empty = build_file_write_perms("{}");

        for t in [from_none, from_empty] {
            let AllOr::Inner(t) = t else {
                panic!("expecting AllOr::Inner");
            };

            assert!(t.files.is_empty());
            assert_eq!(
                t,
                FileWritePermissions {
                    files: HashMap::new()
                },
            );
        }
    }

    #[test]
    fn create_with_files() {
        let abc = build_file_write_perms(r#"{ "a", "b", "c" }"#);
        let ab = build_file_write_perms(r#"{ "a", "b" }"#);
        let ca = build_file_write_perms(r#"{ "c", "a" }"#);

        assert!(abc > ab); // abc contains more files than ab
        assert!(abc > ca); // abc contains more files than ca

        assert!(!(ab >= ca) && !(ab <= ca)); // ab and ca aren't comparable

        let all = build_file_write_perms("\"all\"");
        for t in [abc, ab, ca] {
            assert!(all > t);
        }
    }

    #[test]
    fn create_max_write_opts() {
        let opts = build_file_write_opts("\"all\"");

        assert!(opts.can_create_new());
        assert!(opts.can_overwrite_existing());
        assert!(opts.can_seek());

        assert_eq!(
            opts,
            FileWriteOptions::CanOverwrite {
                create: true,
                mode: OverwriteMode::Truncate
            }
        );
    }

    #[test]
    fn write_opts_comparisons() {
        use FileWriteOptions::*;
        use OverwriteMode::*;

        let new_only = NewOnly;
        let append_only = CanOverwrite {
            create: false,
            mode: AppendOnly,
        };
        let append_only_create = CanOverwrite {
            create: true,
            mode: AppendOnly,
        };
        let append = CanOverwrite {
            create: false,
            mode: Append,
        };
        let append_create = CanOverwrite {
            create: true,
            mode: Append,
        };
        let trunc = CanOverwrite {
            create: false,
            mode: Append,
        };
        let trunc_create = CanOverwrite {
            create: true,
            mode: Append,
        };
        let start = CanOverwrite {
            create: false,
            mode: Append,
        };
        let start_create = CanOverwrite {
            create: true,
            mode: Append,
        };

        assert_eq!(append, trunc);
        assert_eq!(append, start);
        assert_eq!(trunc, start);

        assert_eq!(append_create, trunc_create);
        assert_eq!(append_create, start_create);
        assert_eq!(trunc_create, start_create);

        // the three "max" values should be greater than all others
        for max in [start_create, trunc_create, append_create] {
            for non_max in [
                new_only,
                append_only,
                append_only_create,
                append,
                trunc,
                start,
            ] {
                assert!(max > non_max);
            }
        }

        for noncomparable in [
            (append_only, new_only),
            (new_only, append),
            (new_only, trunc),
            (new_only, start),
            (append_only_create, append),
            (append_only_create, trunc),
            (append_only_create, start),
        ] {
            assert_eq!(
                noncomparable.0.partial_cmp(&noncomparable.1),
                None,
                "{:?} shouldn't compare with {:?}",
                noncomparable.0,
                noncomparable.1
            );
            assert_eq!(noncomparable.1.partial_cmp(&noncomparable.0), None);
        }

        for greater in [
            append_only_create,
            trunc,
            trunc_create,
            append,
            append_create,
            start,
            start_create,
        ] {
            assert!(append_only < greater);
        }

        for greater in [
            append_only_create,
            trunc_create,
            append_create,
            start_create,
        ] {
            assert!(new_only < greater);
        }
    }

    #[test]
    fn full_construction() {
        let test = build_file_write_perms(
            r#"{
                "somefile.txt",
                other_file = "all",
                ["new_file.txt"] = "create_only",

                append_only = {
                    mode = "append_only",
                    -- create = false, -- defaults to false
                },

                truncate_or_create = {
                    mode = "truncate",
                    create = true,
                },

                -- this starts by appending but can seek anywhere to edit the whole file
                append = {
                    mode = "append"
                },
            }"#,
        );

        let expected = FileWritePermissions {
            files: HashMap::from([
                ("somefile.txt".into(), FileWriteOptions::default()),
                (
                    "other_file".into(),
                    FileWriteOptions::CanOverwrite {
                        create: true,
                        mode: OverwriteMode::Truncate,
                    },
                ),
                ("new_file.txt".into(), FileWriteOptions::NewOnly),
                (
                    "append_only".into(),
                    FileWriteOptions::CanOverwrite {
                        create: false,
                        mode: OverwriteMode::AppendOnly,
                    },
                ),
                (
                    "truncate_or_create".into(),
                    FileWriteOptions::CanOverwrite {
                        create: true,
                        mode: OverwriteMode::Truncate,
                    },
                ),
                (
                    "append".into(),
                    FileWriteOptions::CanOverwrite {
                        create: false,
                        mode: OverwriteMode::Append,
                    },
                ),
            ]),
        };

        assert_eq!(test, AllOr::Inner(expected));
    }
}

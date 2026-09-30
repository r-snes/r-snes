//! Filesystem-related permissions

use std::{path::PathBuf, string::FromUtf8Error};

use piccolo::{Context, Value};
use product_order::combine_ordering;

use super::{FilePermissions, PermTreeFromAllOr, PermTreeNode};

/// All different options to open 1 file for reading/writing.
///
/// # Comparisons/Equalities
/// `PartialOrd` and `PartialEq` impls don't necessarily reflect
/// exactly the read/write mode that was requested, but only "how much"
/// the requested permissions allow to do.
///
/// As such, we end with eight equivalence classes:
/// - NewOnly (can only create new files, can't touch existing files)
/// - ReadOnly (reads existing file, no writing, no creating)
/// - AppendOnly + !create (can't create files, can only append to existing)
/// - AppendOnly + create (may create new files but can only append in
///   existing files, not overwrite them fully)
/// - Write + !create + !read (can't create nor read, but can fully overwrite existing files)
/// - Write + !create + read (can't create, but can fully overwrite and read existing files)
/// - Write + create + !read (can create new files, fully overwrite existing, but can't read files)
/// - Write + create + read (can do anything: create new files, fully
///   overwrite existing and read)
///
/// In this list, "Write" corresponds to either [`Append`](SeekingWrite::Append),
/// [`Truncate`](SeekingWrite::Truncate) or [`Start`](SeekingWrite::Start),
/// since they all allow overwriting existing data in opened files
///
/// The ordering of these equivalence classes is described in [the
/// doc of the `PartialOrd impl`](Self#impl-PartialOrd-for-FileWriteOptions)
#[derive(Copy, Clone, Eq, Debug)]
pub enum FileReadWriteOptions {
    /// Only create a new file, don't overwrite
    /// (or even append) an existing file
    NewOnly,

    /// Open an existing file only for reading
    ReadOnly,

    /// May overwrite (at least append) an existing file
    CanOverwrite {
        /// Create bit
        create: bool,
        /// How the file may be overwritten
        mode: OverwriteMode,
    },
}

/// Enum describing how existing files can be overwritten
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum OverwriteMode {
    /// Open in append mode but don't allow seeking at all,
    /// to prevent modification of existing data in the file
    AppendOnly,

    /// All write modes other than AppendOnly are allowed to
    /// seek. Read/Write modes are also under this variant.
    CanSeek { read: bool, mode: SeekingWrite },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SeekingWrite {
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

impl FileReadWriteOptions {
    /// Whether these file read/write options will allow
    /// seeking the opened file
    pub fn can_seek(self) -> bool {
        !matches!(
            self,
            Self::CanOverwrite {
                mode: OverwriteMode::AppendOnly,
                ..
            }
        )
    }

    /// Whether these file read/write options may create
    /// new files on the user's machine
    pub fn can_create_new(self) -> bool {
        matches!(
            self,
            Self::NewOnly | Self::CanOverwrite { create: true, .. },
        )
    }

    /// Whether these file write options may touch
    /// existing files at all
    pub fn can_touch_existing(self) -> bool {
        matches!(self, Self::CanOverwrite { .. })
    }

    /// Whether these file write options may overwrite
    /// existing data in files (just appending counts as false)
    pub fn can_overwrite_existing(self) -> bool {
        matches!(
            self,
            Self::CanOverwrite {
                mode: OverwriteMode::CanSeek { .. },
                ..
            }
        )
    }

    /// Whether these file read/write options can read data in a file
    pub fn can_read(self) -> bool {
        matches!(self, Self::NewOnly) || self.can_read_existing()
    }

    /// Whether these file read/write options may read data
    /// in an existing file
    pub fn can_read_existing(self) -> bool {
        matches!(
            self,
            Self::ReadOnly
                | Self::CanOverwrite {
                    mode: OverwriteMode::CanSeek { read: true, .. },
                    ..
                }
        )
    }

    /// Whether these file read/write options may write
    /// files in any way (append, creating new files, etc.)
    pub fn can_write(self) -> bool {
        !matches!(self, Self::ReadOnly)
    }
}

impl Default for FileReadWriteOptions {
    /// Default options to apply when constructed from
    /// just the file name, or with `"file" = "all"`
    fn default() -> Self {
        Self::CanOverwrite {
            create: true,
            mode: OverwriteMode::CanSeek {
                read: true,
                mode: SeekingWrite::Truncate,
            },
        }
    }
}

impl PartialEq for FileReadWriteOptions {
    fn eq(&self, other: &Self) -> bool {
        self.can_create_new() == other.can_create_new()
            && self.can_touch_existing() == other.can_touch_existing()
            && self.can_overwrite_existing() == other.can_overwrite_existing()
            && self.can_read() == other.can_read()
    }
}

/// Cases should be ordered as per this
/// [Hasse diagram](https://en.wikipedia.org/wiki/Hasse_diagram)
/// ```txt
///         RWC
///        /   \
///       RW   WC
///      /  \ /  \
///     RO   W  AOC
///          |  /|
///          | / |
///          |/  |
///         AO   NO
/// ```
/// (elements which aren't linked "don't compare": neither is greater than
/// the other, but they aren't equal either; for elements which are linked:
/// the one higher than the other is "greater" than the other)
///
/// In this diagram, the eight elements are the equivalence classes
/// described in the [top-level doc for the type](Self#comparisonsequalities):
/// `AO` is append-only, `NO` is new-only, `W` is "write", `AOC` is
/// append-only + create, `WC` is write + create, `RO` is read-only `RW`
/// is write + read, `RWC` is write + read + create.
impl PartialOrd for FileReadWriteOptions {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        product_order::combine_option_orderings(
            [
                Self::can_create_new,
                Self::can_touch_existing,
                Self::can_overwrite_existing,
                Self::can_read_existing,
            ]
            .map(|cmp| cmp(*self).partial_cmp(&cmp(*other))),
        )
    }
}

impl PartialOrd for FilePermissions {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering::*;

        let mut acc = Equal;

        // first compare against files which are in self, which
        // may or may not be in other.
        // in this loop we are sure to catch all files which are in both
        for (filepath, write_options) in &self.files {
            if let Some(other_options) = other.files.get(filepath) {
                // both have the same file, combine with the comparison of
                // read/write options
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

impl PermTreeFromAllOr for FilePermissions {
    fn from_lua_inner<'gc>(ctx: Context<'gc>, value: Value<'gc>) -> Option<Self> {
        let Value::Table(tab) = value else {
            eprintln!("read/write permissions should be a table");
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

                    ret.files.insert(pathbuf, FileReadWriteOptions::from_lua(ctx, v)?);
                }
                _ => eprintln!("unexpected key val combo in file read/write permissions"),
            }
        }
        Some(ret)
    }
}

impl PermTreeNode for FileReadWriteOptions {
    fn from_lua<'gc>(ctx: Context<'gc>, value: Value<'gc>) -> Option<Self> {
        match value {
            Value::String(s) if s.as_bytes() == b"all" => Some(Default::default()),
            Value::String(s) if s.as_bytes() == b"create_only" => Some(Self::NewOnly),
            Value::String(s) if s.as_bytes() == b"read_only" => Some(Self::ReadOnly),
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

                let read = match tab.get_value(ctx, "read") {
                    Value::Boolean(b) => Some(b),
                    Value::Nil => None,
                    _ => {
                        eprintln!("invalid value for 'read' in overwrite options, assuming false");
                        None
                    }
                };

                let mode: OverwriteMode = match tab.get_value(ctx, "mode") {
                    Value::String(s) if s.as_bytes() == b"append_only" => {
                        if read.is_some() {
                            eprintln!("invalid \"read\" node on append_only");
                            return None;
                        }
                        OverwriteMode::AppendOnly
                    }
                    Value::String(s) if s.as_bytes() == b"append" => OverwriteMode::CanSeek {
                        read: read.unwrap_or_default(),
                        mode: SeekingWrite::Append,
                    },
                    Value::String(s) if s.as_bytes() == b"truncate" => OverwriteMode::CanSeek {
                        read: read.unwrap_or_default(),
                        mode: SeekingWrite::Truncate,
                    },
                    Value::String(s) if s.as_bytes() == b"start" => OverwriteMode::CanSeek {
                        read: read.unwrap_or_default(),
                        mode: SeekingWrite::Start,
                    },
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
                eprintln!("invalid value to construct file read/write opts");
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

impl From<FileReadWriteOptions> for std::fs::OpenOptions {
    fn from(options: FileReadWriteOptions) -> Self {
        let mut ret = Self::new();

        ret.read(options.can_read());
        ret.write(options.can_write());
        ret.create(options.can_create_new());
        match options {
            FileReadWriteOptions::NewOnly => {
                ret.create_new(true);
            }
            FileReadWriteOptions::ReadOnly => {}
            FileReadWriteOptions::CanOverwrite { mode, .. } => {
                ret.create_new(false);

                match mode {
                    OverwriteMode::AppendOnly
                    | OverwriteMode::CanSeek {
                        mode: SeekingWrite::Append,
                        ..
                    } => {
                        ret.truncate(false);
                        ret.append(true);
                    }
                    OverwriteMode::CanSeek {
                        mode: SeekingWrite::Truncate,
                        ..
                    } => {
                        ret.truncate(true);
                        ret.append(false);
                    }
                    OverwriteMode::CanSeek {
                        mode: SeekingWrite::Start,
                        ..
                    } => {
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
        perm_tree::{
            FilePermissions,
            filesystem::SeekingWrite::{Append, Start, Truncate},
            test::build_from_lua,
        },
        permission::helpers::AllOr,
    };

    fn build_file_perms(lua: &str) -> AllOr<FilePermissions> {
        build_from_lua(lua, <AllOr<FilePermissions> as PermTreeNode>::from_lua)
            .expect("valid construction")
    }

    fn build_file_write_opts(lua: &str) -> FileReadWriteOptions {
        build_from_lua(lua, FileReadWriteOptions::from_lua).expect("valid construction")
    }

    #[test]
    fn create_with_no_files() {
        let from_none = build_file_perms("\"none\"");
        let from_empty = build_file_perms("{}");

        for t in [from_none, from_empty] {
            let AllOr::Inner(t) = t else {
                panic!("expecting AllOr::Inner");
            };

            assert!(t.files.is_empty());
            assert_eq!(
                t,
                FilePermissions {
                    files: HashMap::new()
                },
            );
        }
    }

    #[test]
    fn create_with_files() {
        let abc = build_file_perms(r#"{ "a", "b", "c" }"#);
        let ab = build_file_perms(r#"{ "a", "b" }"#);
        let ca = build_file_perms(r#"{ "c", "a" }"#);

        assert!(abc > ab); // abc contains more files than ab
        assert!(abc > ca); // abc contains more files than ca

        assert_eq!(ab.partial_cmp(&ca), None); // ab and ca aren't comparable

        let all = build_file_perms("\"all\"");
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
        assert!(opts.can_read());

        assert_eq!(
            opts,
            FileReadWriteOptions::CanOverwrite {
                create: true,
                mode: OverwriteMode::CanSeek {
                    read: true,
                    mode: SeekingWrite::Truncate
                },
            }
        );
    }

    #[test]
    fn write_opts_comparisons() {
        use FileReadWriteOptions::*;
        use OverwriteMode::*;

        let new_only = NewOnly;
        let read_only = ReadOnly;
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
            mode: CanSeek {
                read: false,
                mode: Append,
            },
        };
        let append_create = CanOverwrite {
            create: true,
            mode: CanSeek {
                read: false,
                mode: Append,
            },
        };
        let append_read = CanOverwrite {
            create: false,
            mode: CanSeek {
                read: true,
                mode: Append,
            },
        };
        let append_read_create = CanOverwrite {
            create: true,
            mode: CanSeek {
                read: true,
                mode: Append,
            },
        };
        let trunc = CanOverwrite {
            create: false,
            mode: CanSeek {
                read: false,
                mode: Truncate,
            },
        };
        let trunc_create = CanOverwrite {
            create: true,
            mode: CanSeek {
                read: false,
                mode: Truncate,
            },
        };
        let trunc_read = CanOverwrite {
            create: false,
            mode: CanSeek {
                read: true,
                mode: Truncate,
            },
        };
        let trunc_read_create = CanOverwrite {
            create: true,
            mode: CanSeek {
                read: true,
                mode: Truncate,
            },
        };
        let start = CanOverwrite {
            create: false,
            mode: CanSeek {
                read: false,
                mode: Start,
            },
        };
        let start_create = CanOverwrite {
            create: true,
            mode: CanSeek {
                read: false,
                mode: Start,
            },
        };
        let start_read = CanOverwrite {
            create: false,
            mode: CanSeek {
                read: true,
                mode: Start,
            },
        };
        let start_read_create = CanOverwrite {
            create: true,
            mode: CanSeek {
                read: true,
                mode: Start,
            },
        };

        // test equivalence classes
        assert_eq!(append, trunc);
        assert_eq!(append, start);
        assert_eq!(trunc, start);

        assert_eq!(append_create, trunc_create);
        assert_eq!(append_create, start_create);
        assert_eq!(trunc_create, start_create);

        assert_eq!(append_read, trunc_read);
        assert_eq!(append_read, start_read);
        assert_eq!(trunc_read, start_read);

        assert_eq!(append_read_create, trunc_read_create);
        assert_eq!(append_read_create, start_read_create);
        assert_eq!(trunc_read_create, start_read_create);

        // the three "max" values should be greater than all others
        for max in [start_read_create, trunc_read_create, append_read_create] {
            for non_max in [
                start,
                start_read,
                start_create,
                trunc,
                trunc_read,
                trunc_create,
                append,
                append_read,
                append_create,
                append_only,
                append_only_create,
                new_only,
                read_only,
            ] {
                assert!(max > non_max, "MAX {max:?} should be > non-max {non_max:?}")
            }
        }

        // we have 10 "pairs" of equivalence classes which don't compare:
        // (AO, NO), (NO, W), and (AOC, W), (AOC, RW), (RO, AO),
        // (RO, AOC), (RO, NO), (RO, W), (RO, WC), (RW, WC)
        for noncomparable in [
            // AO,NO
            (append_only, new_only),
            // NO, W
            (new_only, append),
            (new_only, trunc),
            (new_only, start),
            // AOC, W
            (append_only_create, append),
            (append_only_create, trunc),
            (append_only_create, start),
            // AOC, RW
            (append_only_create, append_read),
            (append_only_create, trunc_read),
            (append_only_create, start_read),
            // RO, AO
            (read_only, append_only),
            // RO, AOC
            (read_only, append_only_create),
            // RO, NO
            (read_only, new_only),
            // RO, W
            (read_only, append),
            (read_only, trunc),
            (read_only, start),
            // RO, WC
            (read_only, append_create),
            (read_only, trunc_create),
            (read_only, start_create),
            // RW, WC
            (append_read, append_create),
            (append_read, start_create),
            (append_read, trunc_create),
            (start_read, append_create),
            (start_read, start_create),
            (start_read, trunc_create),
            (trunc_read, append_create),
            (trunc_read, start_create),
            (trunc_read, trunc_create),
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

        // WC values have 4 eq classes below them: AO, NO, W, AOC
        for wc in [start_create, trunc_create, append_create] {
            for non_max in [
                new_only,
                append_only,
                append_only_create,
                append,
                trunc,
                start,
            ] {
                assert!(wc > non_max, "{wc:?} should be greater than {non_max:?}");
            }
        }

        // AOC, W and WC should be greater than AO
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

        // only AOC and WC are greater than NO
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
    fn readonly_has_correct_values() {
        let read = FileReadWriteOptions::ReadOnly;

        assert!(!read.can_write());
        assert!(!read.can_overwrite_existing());
        assert!(!read.can_touch_existing());
        assert!(!read.can_create_new());
        assert!(read.can_seek());
        assert!(read.can_read());
    }

    #[test]
    fn full_construction() {
        let test = build_file_perms(
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
                    mode = "append",
                    read = true,
                },

                rdonly = "read_only",
            }"#,
        );

        let expected = FilePermissions {
            files: HashMap::from([
                ("somefile.txt".into(), FileReadWriteOptions::default()),
                (
                    "other_file".into(),
                    FileReadWriteOptions::CanOverwrite {
                        create: true,
                        mode: OverwriteMode::CanSeek {
                            read: true,
                            mode: SeekingWrite::Truncate,
                        },
                    },
                ),
                ("new_file.txt".into(), FileReadWriteOptions::NewOnly),
                (
                    "append_only".into(),
                    FileReadWriteOptions::CanOverwrite {
                        create: false,
                        mode: OverwriteMode::AppendOnly,
                    },
                ),
                (
                    "truncate_or_create".into(),
                    FileReadWriteOptions::CanOverwrite {
                        create: true,
                        mode: OverwriteMode::CanSeek {
                            read: false,
                            mode: SeekingWrite::Truncate,
                        },
                    },
                ),
                (
                    "append".into(),
                    FileReadWriteOptions::CanOverwrite {
                        create: false,
                        mode: OverwriteMode::CanSeek {
                            read: true,
                            mode: SeekingWrite::Append,
                        },
                    },
                ),
                ("rdonly".into(), FileReadWriteOptions::ReadOnly),
            ]),
        };

        assert_eq!(test, AllOr::Inner(expected));
    }
}

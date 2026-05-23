use strict_partial_ord_derive as strict;
use crate::permission::Permission;
use permission_derive_macro::Permission;

/// + internal // access stuff within the emulator
/// | + control // control the emulator, not the components
/// | | + dialog // allows the plugin to show dialog windows
/// | | ` pause // pause/resume game
/// | |
/// | + cpu
/// | | ` registers
/// | |
/// | + ppu // access framebuffer, loaded objects, etc.
/// | | ` display // draw to the framebuffer
/// | |
/// | ` bus // interact with memory
/// |   + read
/// |   ` write
/// |
/// ` external // access to the host system
///   + filesystem
///   | + read_file
///   | ` write_file
///   |
///   ` http

#[derive(PartialEq, Eq, strict::PartialOrd, Permission)]
pub struct RSnesPermissions {
    pub internal: InternalPermissions,
    pub external: ExternalPermissions,
}

#[derive(PartialEq, Eq, strict::PartialOrd, Permission)]
pub struct InternalPermissions {
    pub control: ControlPermissions,
    pub cpu: CpuPermissions,
    pub ppu: PpuPermissions,
    pub bus: BusPermissions,
}

#[derive(PartialEq, Eq, strict::PartialOrd, Permission)]
pub struct ControlPermissions {
    pub dialog: bool,
    pub pause: bool,
}

#[derive(PartialEq, Eq, strict::PartialOrd, Permission)]
pub struct CpuPermissions {
    pub registers: bool,
}

#[derive(PartialEq, Eq, strict::PartialOrd, Permission)]
pub struct PpuPermissions {
    pub display: bool,
}

#[derive(PartialEq, Eq, strict::PartialOrd, Permission)]
pub struct BusPermissions {
    pub read: bool,
    pub write: bool,
}

#[derive(PartialEq, Eq, strict::PartialOrd, Permission)]
pub struct ExternalPermissions {
    pub filesystem: FileSystemPermissions,
    pub http: bool,
}

#[derive(PartialEq, Eq, strict::PartialOrd, Permission)]
pub struct FileSystemPermissions {
    pub read: bool,
    pub write: bool,
}


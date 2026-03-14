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

pub struct RSnesPermissions {
    pub internal: InternalPermissions,
    pub external: ExternalPermissions,
}

pub struct InternalPermissions {
    pub control: ControlPermissions,
    pub cpu: CpuPermissions,
    pub ppu: PpuPermissions,
    pub bus: BusPermissions,
}

pub struct ControlPermissions {
    pub dialog: bool,
    pub pause: bool,
}

pub struct CpuPermissions {
    pub registers: bool,
}

pub struct PpuPermissions {
    pub display: bool,
}

pub struct BusPermissions {
    pub read: bool,
    pub write: bool,
}

pub struct ExternalPermissions {
    pub filesystem: FileSystemPermissions,
    pub http: bool,
}

pub struct FileSystemPermissions {
    pub read: bool,
    pub write: bool,
}


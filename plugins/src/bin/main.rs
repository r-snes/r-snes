use plugins::plugin::{
    Plugin,
    PluginPermRequest,
};
use eframe::egui;

struct PermRequestApp<'app, 'req> {
    title: String,
    perm_request: &'app mut PluginPermRequest<'req>,
}

impl<'app, 'req> PermRequestApp<'app, 'req> {
    fn new(perm_request: &'app mut PluginPermRequest<'req>, title: String) -> Self {
        Self {
            perm_request,
            title,
        }
    }
}

impl eframe::App for PermRequestApp<'_, '_> {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading(&self.title);
            self.perm_request.show_gui(ui);
        });
    }
}

fn main() -> eframe::Result {
    // Load the Lua file
    let filename = "./plugin.lua";
    let plugin = Plugin::load_from_file(std::path::Path::new(filename));

    let mut plugin = match plugin {
        Err(e) => {
            dbg!(e);
            std::process::exit(1);
        }
        Ok(p) => p
    };

    env_logger::init();

    let eframe_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    let title = format!("Loading plugin {}", plugin.path.as_ref().unwrap().display());
    let mut perm_request = plugin.perm_request();

    eframe::run_native(
        &format!("r-snes | {title}"),
        eframe_options,
        Box::new(|_| Ok(Box::new(PermRequestApp::new(&mut perm_request, title)))),
    )?;

    if perm_request.allow_all {
        println!("\x1B[1;32m✔\x1B[0m permissions granted");
    } else {
        println!("\x1B[1;31m✘\x1B[0m permission denied, exiting");
        return Ok(());
    }

    if let Some(init) = plugin.table.init {
        let ex = plugin.lua.enter(|ctx| {
            let closure = ctx.fetch(&init);
            let ex = piccolo::Executor::start(ctx, closure.into(), ());

            ctx.stash(ex)
        });

        plugin.lua.execute::<()>(&ex);
    }

    Ok(())
}

use plugins::plugin::Plugin;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the Lua file
    let filename = "./plugin.lua";
    let p = Plugin::load(std::path::Path::new(filename));

    match p {
        Err(e) => {
            dbg!(e);
        }
        Ok(p) => {
            dbg!(p.table);
        }
    }

    Ok(())
}

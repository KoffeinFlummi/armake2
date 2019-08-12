use clap;
use hashbrown::HashMap;

use armake2::Command;
use armake2::error::PrintableError;

fn main() {
    let mut version = env!("CARGO_PKG_VERSION").to_string();
    if cfg!(debug_assertions) {
        version.push_str("-debug");
    }

    let mut app = clap::App::new("armake2")
        .version(version.as_ref())
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"));

    let mut commands: Vec<Box<dyn Command>> = Vec::new();
    let mut hash_commands: HashMap<String, &Box<dyn Command>> = HashMap::new();

    commands.push(Box::new(armake2::commands::Inspect {}));
    commands.push(Box::new(armake2::commands::Cat {}));
    commands.push(Box::new(armake2::commands::Binarize {}));
    commands.push(Box::new(armake2::commands::Rapify {}));
    commands.push(Box::new(armake2::commands::Derapify {}));
    commands.push(Box::new(armake2::commands::Pack {}));
    commands.push(Box::new(armake2::commands::Unpack {}));
    commands.push(Box::new(armake2::commands::Build {}));

    #[cfg(feature = "signing")]
    {
        commands.push(Box::new(armake2::commands::signing::Keygen {}));
        commands.push(Box::new(armake2::commands::signing::Sign {}));
        commands.push(Box::new(armake2::commands::signing::Verify {}));
    }

    for command in commands.iter() {
        let sub = command.register();
        hash_commands.insert(sub.get_name().to_owned(), command);
        app = app.subcommand(sub);
    }

    let matches = app.get_matches();

    match matches.subcommand_name() {
        Some(v) => {
            match hash_commands.get(v) {
                Some(c) => {
                    let sub_matches = matches.subcommand_matches(v).unwrap();
                    c.run(sub_matches).unwrap_or_print();
                },
                None => println!("Unknown Command"),
            }
        },
        None => println!("No command"),
    }
}

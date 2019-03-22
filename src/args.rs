use clap::{App, Arg};

use crate::command::Command;

const AFTER_HELP: &str = r"EXAMPLES:
    Bind mount /etc/dir (TARGET) directory having the BIND_ROOT /overlay (SOURCE). The tool will
    create an empty directory /overlay/etc/dir (if not available) and bind mount /overlay/etc/dir
    in /etc/dir. 

    Bind mount /etc/file (TARGET) file having the BIND_ROOT /overlay (SOURCE). The tool will create
    an empty file /overlay/etc/file (if not available) and bind mount /overlay/etc/file in
    /etc/file.

    In case of shadowing (TARGET directory/file is not empty) the tool warns and proceeds with
    mount.";

pub struct Arguments {
    pub command: Command,
    pub target: String,
    pub bind_root: String,
}

impl Arguments {
    pub fn new() -> Arguments {
        let matches = App::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .after_help(AFTER_HELP)
            .arg(
                Arg::with_name("command")
                    .long("command")
                    .takes_value(true)
                    .possible_values(&[Command::Mount.as_ref(), Command::Unmount.as_ref()])
                    .default_value("mount")
                    .required(true),
            )
            .arg(
                Arg::with_name("bind-root")
                    .long("bind-root")
                    .takes_value(true)
                    .required(true),
            )
            .arg(
                Arg::with_name("target")
                    .long("target")
                    .takes_value(true)
                    .required(true),
            )
            .get_matches();

        let command = matches
            .value_of("command")
            .unwrap()
            .parse::<Command>()
            .expect("Fix clap arguments");
        let bind_root = matches.value_of("bind-root").expect("Fix clap arguments");
        let target = matches
            .value_of("target")
            .expect("Fix clap arguments")
            .trim_end_matches('/');

        Arguments {
            command,
            target: target.to_string(),
            bind_root: bind_root.to_string(),
        }
    }
}

use std::env;
use std::process::exit;
use std::path::Path;
use std::fs;
use std::ffi::CString;
use std::io;
use std::io::Read;

extern crate errno;
extern crate libc;

enum Command {
    Mount,
    Umount,
}

fn help() {
    println!(
        "
  This tools bind mounts a path to another path keeping the same filesystem structure of the
  TARGET in the SOURCE. As well it takes of care creating the SOURCE for the case where
  TARGET is a file or a directory.

  Examples:

    Bind mount /etc/dir (TARGET) directory having the BIND_ROOT /overlay (SOURCE). The tool will
    create an empty directory /overlay/etc/dir (if not available) and bind mount /overlay/etc/dir
    in /etc/dir.

    Bind mount /etc/file (TARGET) file having the BIND_ROOT /overlay (SOURCE). The tool will create
    an empty file /overlay/etc/file (if not available) and bind mount /overlay/etc/file in
    /etc/file.

  In case of shadowing (TARGET directory/file is not empty) the tool warns and proceeds with
  mount.

  Usage: bindmount --target TARGET --bind-root BIND_ROOT [--command COMMAND]

  Flags:
    --help
      Print this message.
    --target TARGET
      The TARGET path for the mount. Based on this and the BIND_ROOT (see below), the tool computes
      the SOURCE.
      Required argument.
      Example: /foo/bar which will use the SOURCE as BIND_ROOT/foo/bar.
    --command mount|unmount
      The command we want to run on the bind mount.
        'mount'    - mount the bind mount
        'unmount'  - unmount the bind mount
      When not provided mount is assumed.
    --bind-root BIND_ROOT
      The root directory of bind mounts.
      Required argument."
    )
}

fn path_is_mounted(p: &Path) -> Result<bool, &str> {
    match fs::File::open("/proc/mounts") {
        Err(_) => Err("Failed to open /proc/mounts"),
        Ok(f) => {
            let mut buf_reader = std::io::BufReader::new(f);
            let mut contents = String::new();
            match buf_reader.read_to_string(&mut contents) {
                Err(_) => Err("Failed to read /proc/mounts"),
                Ok(_) => {
                    for (_, l) in contents.lines().enumerate() {
                        let dst = l.split_whitespace().nth(1).unwrap();
                        if dst == p.to_str().unwrap() {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
            }
        }
    }
}

fn dir_is_empty(p: &Path) -> Result<bool, &str> {
    for _ in fs::read_dir(p).unwrap() {
        return Ok(false);
    }
    Ok(true)
}

fn file_is_empty(p: &Path) -> Result<bool, &str> {
    match fs::symlink_metadata(p) {
        Err(e) => panic!("{:?}", e),
        Ok(p_meta) => if p_meta.len() == 0 {
            Ok(true)
        } else {
            Ok(false)
        },
    }
}

fn entry_is_empty(p: &Path) -> Result<bool, &str> {
    match fs::symlink_metadata(p) {
        Err(e) => panic!("{:?}", e),
        Ok(p_meta) => {
            let p_file_type = p_meta.file_type();
            if p_file_type.is_dir() {
                dir_is_empty(p)
            } else if p_file_type.is_file() {
                file_is_empty(p)
            } else {
                Err("Not implemented")
            }
        }
    }
}

fn create_dir_all_racy(p: &Path) -> io::Result<()> {
    while let Err(e) = fs::create_dir_all(p) {
        if e.kind() != io::ErrorKind::AlreadyExists {
            return Err(e);
        }
    }
    Ok(())
}

fn create_mountpoint(p: &Path, t: &fs::FileType) -> io::Result<()> {
    if p.exists() {
        return Ok(());
    }
    if t.is_dir() {
        create_dir_all_racy(p)?;
    } else if t.is_file() {
        create_dir_all_racy(p.parent().unwrap())?;
        fs::File::create(p)?;
    } else {
        return Err(std::io::Error::new(io::ErrorKind::InvalidInput, ""));
    }
    unsafe {
        println!("INFO: Created {}, sync filesystems...", p.display());
        libc::sync();
    }
    Ok(())
}

fn main() {
    let mut c: Command = Command::Mount;
    let mut t = String::new();
    let mut r = String::new();
    let args: Vec<String> = env::args().collect();
    for (n, arg) in args.iter().enumerate().skip(1) {
        if !arg.starts_with("--") {
            continue;
        }
        match arg.as_ref() {
            "--command" => if args.len() - 1 > n {
                match args[n + 1].as_ref() {
                    "mount" => c = Command::Mount,
                    "unmount" => c = Command::Umount,
                    _ => {
                        println!("ERROR: Not a valid argument for --command.\n");
                        help();
                        exit(1);
                    }
                }
            } else {
                println!("ERROR --command flag needs an argument.\n");
                help();
                exit(1);
            },
            "--target" => if args.len() - 1 > n {
                t = String::from(args[n + 1].as_str());
            } else {
                println!("ERROR --target flag needs an argument.\n");
                help();
                exit(1);
            },
            "--bind-root" => if args.len() - 1 > n {
                r = String::from(args[n + 1].as_str());
            } else {
                println!("ERROR --bind-root flag needs an argument.\n");
                help();
                exit(1);
            },
            "--help" => {
                help();
                exit(0);
            }
            unknown => {
                println!("ERROR: No such flag: {}.\n", unknown);
                help();
                exit(1);
            }
        }
    }

    if t.is_empty() {
        println!("ERROR: TARGET not provided.\n");
        help();
        exit(1);
    }

    if r.is_empty() {
        println!("ERROR: Bind root path not provided.\n");
        help();
        exit(1);
    }

    if t.ends_with("/") {
        t.pop();
    }
    let root_mountpoint = Path::new("/").join(&t.as_str());
    let bind_mountpoint = Path::new(&r.as_str()).join(&root_mountpoint.strip_prefix("/").unwrap());

    let is_mounted = match path_is_mounted(&root_mountpoint) {
        Err(e) => {
            println!(
                "ERROR: Could not check if {} is mounted: {}.",
                bind_mountpoint.display(),
                e
            );
            exit(1);
        }
        Ok(mounted) => mounted,
    };

    match c {
        Command::Mount => {
            println!(
                "INFO: Bindmounting {} in {} ...",
                root_mountpoint.display(),
                bind_mountpoint.display()
            );

            if !root_mountpoint.exists() {
                println!(
                    "ERROR: {} doesn't exist. Nothing to mount.",
                    root_mountpoint.display()
                );
                exit(1);
            }

            if is_mounted {
                println!("INFO: bind mountpont is already mounted.");
                exit(0);
            }
        }
        Command::Umount => {
            println!("INFO: Unmounting {} ...", bind_mountpoint.display());

            if is_mounted {
                unsafe {
                    let ret = libc::umount(
                        CString::new(root_mountpoint.to_str().unwrap())
                            .unwrap()
                            .as_ptr(),
                    );
                    if ret == 0 {
                        println!("INFO: Successfully unmounted {}.", bind_mountpoint.display());
                        exit(0);
                    } else {
                        println!(
                            "ERROR: Failed to unmount {}: {}.",
                            bind_mountpoint.display(),
                            errno::errno()
                        );
                        exit(1);
                    }
                }
            } else {
                println!("INFO: bind mountpont is already unmounted.");
                exit(0);
            }
        }
    }

    // Carry on with mounting - unmounting is completely handled above
    match entry_is_empty(&root_mountpoint) {
        Err(e) => println!("WARN: Check if root mountpoint is empty failed: {}.", e),
        Ok(empty) => if !empty {
            println!(
                "WARN: {} is not an empty entry. You are going to shadow content.",
                root_mountpoint.display()
            );
        },
    }
    match create_mountpoint(
        &bind_mountpoint,
        &root_mountpoint.symlink_metadata().unwrap().file_type(),
    ) {
        Err(e) => {
            println!("ERROR: Could not create bind mountpoint: {}.", e);
            exit(1);
        }
        Ok(()) => {}
    }
    unsafe {
        let ret = libc::mount(
            CString::new(bind_mountpoint.to_str().unwrap())
                .unwrap()
                .as_ptr(),
            CString::new(root_mountpoint.to_str().unwrap())
                .unwrap()
                .as_ptr(),
            CString::new("ext2").unwrap().as_ptr(),
            libc::MS_BIND,
            0 as *mut libc::c_void,
        );
        if ret == 0 {
            println!("INFO: Successfully mounted {}.", bind_mountpoint.display());
        } else {
            println!(
                "ERROR: Failed to mount {}: {}.",
                bind_mountpoint.display(),
                errno::errno()
            );
            exit(1);
        }
    }
}

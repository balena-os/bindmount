use std::env;
use std::process::exit;
use std::path::Path;
use std::fs;
use std::ffi::CString;
use std::io::Read;

extern crate errno;
extern crate libc;

static BIND_ROOT_DEFAULT: &'static str = "/mnt/state/root-overlay";

enum Command {
	Mount,
	Umount,
}

fn help() {
    println!("Usage: ro-state-bindmount --p bind_path [--c command]

  Flags:
    --help
      Print this message.
    --p bind_relative_path
      Bind mount path relative to {0}.
      Example: /foo/bar which will use the bind mount {0}/foo/bar.
    --c mount|unmount
      The command we want to run on the bind mount.
        'mount'    - mount the bind mount path computed based on '--p'.
        'unmount'  - unmount the bind mount path computed based on '--p'
    --bind-root
      The root directory of bind mounts.
      Default: {0}.

", BIND_ROOT_DEFAULT);
}

fn path_is_mounted(p: &Path) -> Result <bool, &str> {
	match fs::File::open("/proc/mounts") {
		Err(_) => {
			Err("Failed to open /proc/mounts")
		},
		Ok(f) => {
			let mut buf_reader = std::io::BufReader::new(f);
			let mut contents = String::new();
			match buf_reader.read_to_string(&mut contents) {
				Err(_) => {
					Err("Failed to read /proc/mounts")
				},
				Ok(_) => {
					for (_, l) in contents.lines().enumerate() {
						let dst = l.split_whitespace().nth(1).unwrap();
						if dst == p.to_str().unwrap() { return Ok(true); }
					}
					Ok(false)
				}
			}
		}
	}
}

fn dir_is_empty(p: &Path) -> Result <bool, & str>  {
	for _ in fs::read_dir(p).unwrap() {
		return Ok(false);
	}
	Ok(true)
}

fn file_is_empty(p: &Path) -> Result <bool, & str> {
	match fs::symlink_metadata(p) {
		Err(e) => panic!("{:?}",e),
		Ok(p_meta) => {
			if p_meta.len() == 0 { Ok(true) }
			else { Ok(false) }
		}
	}
}

fn entry_is_empty(p: &Path) -> Result <bool, & str> {
	match fs::symlink_metadata(p) {
		Err(e) => panic!("{:?}", e),
		Ok(p_meta) => {
			let p_file_type = p_meta.file_type();
			if p_file_type.is_dir() { dir_is_empty(p) }
			else if p_file_type.is_file() { file_is_empty(p) }
			else { Err("Not implemented") }
		}
	}
}

fn create_mountpoint<'a>(p: &Path, t: &fs::FileType) -> Result <bool, &'a str> {
	if p.exists() {
		Ok(false)
	} else {
		if t.is_dir() {
			match fs::create_dir_all(p) {
				Err(_) =>
					return Err("Can't create directories"),
				Ok(_) => return Ok(true)
			}
		} else if t.is_file() {
			if ! p.parent().unwrap().exists() {
				match fs::create_dir_all(p.parent().unwrap()) {
					Err(_) =>
						return Err("Can't create directories"),
					Ok(_) => ()
				}
			}
			match fs::File::create(p) {
				Err(_) => Err("Could not create file."),
				Ok(_) => Ok(true)
			}
		} else { return Err("Not a directory or a file"); }
	}
}

fn main() {
	let mut c: Command = Command::Mount;
	let mut p = String::new();
	let mut r = String::from(BIND_ROOT_DEFAULT);
	let args: Vec<String> = env::args().collect();
	for (n, arg) in args.iter().enumerate().skip(1) {
		if ! arg.starts_with("--") { continue; }
		match arg.as_ref() {
			"--c" => {
				if args.len() - 1  > n {
					match args[n+1].as_ref() {
						"mount" => c = Command::Mount,
						"unmount" => c = Command::Umount,
						_ => {
							println!("ERROR: Not a valid argument for --c.\n");
							help();
							exit(1);
						}
					}
				} else {
					println!("ERROR --c flag needs an argument.\n");
					help();
					exit(1);
				}
			},
			"--p" => {
				if args.len() -1  > n {
					p = String::from(args[n+1].as_str());
				} else {
					println!("ERROR --p flag needs an argument.\n");
					help();
					exit(1);
				}
			},
			"--bind-root" => {
				if args.len() -1  > n {
					r = String::from(args[n+1].as_str());
				} else {
					println!("ERROR --bind-root flag needs an argument.\n");
					help();
					exit(1);
				}
			},
			"--help" => {
				help();
				exit(0);
			},
			unknown => {
				println!("ERROR: No such flag: {}.\n", unknown);
				help();
				exit(1);
			},
		}
	}

	if p.is_empty() {
		println!("ERROR: Path not provided.\n");
		help();
		exit(1);
	}

	let root_mountpoint = Path::new("/").join(&p.as_str());
	let bind_mountpoint = Path::new(&r.as_str()).join(&root_mountpoint.strip_prefix("/").unwrap());


	let is_mounted = path_is_mounted(&bind_mountpoint);
	match c {
		Command::Mount => {
			println!("INFO: Bindmounting {} in {} ...",
			root_mountpoint.display(),
			bind_mountpoint.display());

			if ! root_mountpoint.exists() {
				println!("ERROR: {} doesn't exist. Nothing to mount.",
				root_mountpoint.display());
				exit(1);
			}

			match is_mounted {
				Err(e) => {
					println!("ERROR: Could not check of bind mountpoint is mounted: {}.", e);
					exit(1);
				},
				Ok(mounted) => {
					if mounted == true {
						println!("INFO: bind mountpont is already mounted.");
						exit(0);
					}
				}
			}
		},
		Command::Umount => {
			println!("INFO: Unmounting {} ...", bind_mountpoint.display());

			match is_mounted {
				Err(e) => {
					println!("ERROR: Could not check of bind mountpoint is mounted: {}.", e);
					exit(1);
				},
				Ok(mounted) => {
					if mounted == true {
						unsafe {
							let ret = libc::umount(
								CString::new(bind_mountpoint.to_str().unwrap()).unwrap().as_ptr());
							if ret == 0 {
								println!("INFO: Successfully unmounted {}", bind_mountpoint.display());
								exit(0);
							} else {
								println!("ERROR: Failed to unmount {}: {}.", bind_mountpoint.display(), errno::errno());
								exit(1);
							}
						}
					} else {
						println!("INFO: bind mountpont is already unmounted.");
						exit(0);
					}
				}
			}
		},
	}

	match entry_is_empty(&root_mountpoint) {
		Err(e) => println!("WARN: Check if root mountpoint is empty failed: {}.", e),
		Ok(empty) => {
			if ! empty {
				println!("WARN: {} is not an empty entry. You are going to shadow content.", root_mountpoint.display());
			}
		}
	}
	match create_mountpoint(&bind_mountpoint, &root_mountpoint.symlink_metadata().unwrap().file_type()) {
		Err(e) => {
			println!("ERROR: Could not create bind mountpoint: {}.", e);
			exit(1);
		},
		Ok(wrote) =>  {
			if wrote {
				println!("INFO: Created {} mountpoint.", bind_mountpoint.display());
				unsafe {
					println!("INFO: Sync filesystems...");
					libc::sync();
				}
			} else {
				println!("INFO: {} mountpoint already in place.", bind_mountpoint.display());
			}
		}
	}
	unsafe {
		let ret = libc::mount(
			CString::new(root_mountpoint.to_str().unwrap()).unwrap().as_ptr(),
			CString::new(bind_mountpoint.to_str().unwrap()).unwrap().as_ptr(),
			CString::new("ext2").unwrap().as_ptr(),
			libc::MS_BIND,
			0 as *mut libc::c_void);
		if ret == 0 {
			println!("INFO: Successfully mounted {}", bind_mountpoint.display());
		} else {
			println!("ERROR: Failed to mount {}: {}.", bind_mountpoint.display(), errno::errno());
			exit(1);
		}
	}
}

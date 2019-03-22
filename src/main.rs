use std::{
    ffi::CString,
    fs,
    io::{self, BufRead, BufReader, Write},
    path::Path,
};

use log::{error, info, warn};

use args::Arguments;
use command::Command;
use error::{Error, Result};

mod args;
mod command;
mod error;

fn is_path_mounted(path: &Path) -> Result<bool> {
    let file =
        fs::File::open("/proc/mounts").map_err(|e| Error::io("Failed to open /proc/mounts", e))?;

    let path = path
        .to_str()
        .ok_or_else(|| format!("Invalid path: {:?}", path))?;

    for line in BufReader::new(file).lines() {
        let line = line.map_err(|e| Error::io("Failed to read /proc/mounts", e))?;

        let dst = line
            .split_whitespace()
            .nth(1)
            .ok_or_else(|| format!("Unsupported /proc/mounts entry: {}", line))?;

        if dst == path {
            return Ok(true);
        }
    }

    Ok(false)
}

fn is_dir_empty(path: &Path) -> Result<bool> {
    Ok(fs::read_dir(path)?.next().is_none())
}

fn is_file_empty(path: &Path) -> Result<bool> {
    Ok(fs::symlink_metadata(path)?.len() == 0)
}

fn is_entry_empty(path: &Path) -> Result<bool> {
    let meta = fs::symlink_metadata(path)?;

    let file_type = meta.file_type();
    if file_type.is_dir() {
        is_dir_empty(path)
    } else if file_type.is_file() {
        is_file_empty(path)
    } else {
        Err(format!("Unsupported file type: {:?}", file_type))?
    }
}

fn create_dir_all_racy(p: &Path) -> Result<()> {
    while let Err(e) = fs::create_dir_all(p) {
        if e.kind() != io::ErrorKind::AlreadyExists {
            Err(e)?;
        }
    }
    Ok(())
}

fn create_mountpoint(path: &Path, root_mountpoint: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    let file_type = root_mountpoint.symlink_metadata()?.file_type();

    if file_type.is_dir() {
        create_dir_all_racy(path)?;
    } else if file_type.is_file() {
        create_dir_all_racy(path.parent().unwrap())?;
        fs::File::create(path)?;
    } else {
        Err(format!("Unsupported file type: {:?}", file_type))?;
    }

    info!("Created {}, sync filesystems...", path.display());
    unsafe {
        libc::sync();
    };
    Ok(())
}

fn warn_if_mountpoint_not_empty(path: &Path) {
    match is_entry_empty(path) {
        Err(e) => warn!("Check if root mountpoint is empty failed: {}", e),
        Ok(empty) => {
            if !empty {
                warn!(
                    "{} is not empty. You are going to shadow content.",
                    path.display()
                );
            }
        }
    };
}

fn mount(is_mounted: bool, root_mountpoint: &Path, bind_mountpoint: &Path) -> Result<()> {
    info!(
        "Bindmounting {} in {} ...",
        root_mountpoint.display(),
        bind_mountpoint.display()
    );

    if !root_mountpoint.exists() {
        Err(format!(
            "Root mount point doesn't exist: {}",
            root_mountpoint.display()
        ))?;
    }

    if is_mounted {
        info!("Bind mountpoint is already mounted");
        return Ok(());
    }

    warn_if_mountpoint_not_empty(&root_mountpoint);

    create_mountpoint(&bind_mountpoint, &root_mountpoint)
        .map_err(|e| format!("Could not create bind mountpoint: {}", e))?;

    let c_bind_mountpoint = CString::new(bind_mountpoint.to_str().unwrap()).unwrap();
    let c_root_mountpoint = CString::new(root_mountpoint.to_str().unwrap()).unwrap();
    let c_ext2 = CString::new("ext2").unwrap();

    let ret = unsafe {
        libc::mount(
            c_bind_mountpoint.as_ptr(),
            c_root_mountpoint.as_ptr(),
            c_ext2.as_ptr(),
            libc::MS_BIND,
            std::ptr::null_mut(),
        )
    };

    if ret != 0 {
        let err = format!(
            "Failed to mount {}: {}",
            bind_mountpoint.display(),
            errno::errno()
        );
        Err(err)?;
    }

    info!("Successfully mounted {}", bind_mountpoint.display());
    Ok(())
}

fn unmount(is_mounted: bool, root_mountpoint: &Path, bind_mountpoint: &Path) -> Result<()> {
    info!("Unmounting {} ...", bind_mountpoint.display());

    if !is_mounted {
        info!("Bind mountpoint is already unmounted");
        return Ok(());
    }

    let c_path = CString::new(root_mountpoint.to_str().unwrap()).unwrap();
    let ret = unsafe { libc::umount(c_path.as_ptr()) };

    if ret != 0 {
        Err(format!(
            "Failed to unmount {}: {}",
            bind_mountpoint.display(),
            errno::errno()
        ))?;
    }

    info!("Successfully unmounted {}", bind_mountpoint.display());
    Ok(())
}

fn run() -> Result<()> {
    env_logger::builder()
        .default_format_timestamp(false)
        .format(|buf, record| writeln!(buf, "{}: {}", record.level(), record.args()))
        .filter_level(log::LevelFilter::Info)
        .init();

    let args = Arguments::new();

    let root_mountpoint = Path::new("/").join(&args.target);
    let bind_mountpoint =
        Path::new(&args.bind_root).join(&root_mountpoint.strip_prefix("/").unwrap());

    let is_mounted = is_path_mounted(&root_mountpoint)?;

    match args.command {
        Command::Mount => mount(is_mounted, &root_mountpoint, &bind_mountpoint),
        Command::Unmount => unmount(is_mounted, &root_mountpoint, &bind_mountpoint),
    }
}

fn main() {
    // Reason for this is that the `main` function can return `Result`, but
    // it is ugly formatted - it uses `Debug` instead of `Display`.
    //
    // You do not want to see `Error: Error { msg: \"foo\" } in the console.
    // There're some proposals (nightly) to fix this with the possibility
    // that the `Error` can carry exit status code as well.
    //
    // Till it will be stabilized, lets use this dance.
    if let Err(e) = run() {
        error!("{}", e);
        std::process::exit(1);
    }
}

```
This tools bind mounts a path to another path keeping the same filesystem
structure of the TARGET in the SOURCE. As well it takes of care creating
the SOURCE for the case where TARGET is a file or a directory

USAGE:
    bindmount --bind-root <bind-root> --command <command> --target <target>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --bind-root <bind-root>    
        --command <command>         [default: mount]
                                    [possible values: mount, unmount]
        --target <target>          

EXAMPLES:
    Bind mount /etc/dir (TARGET) directory having the BIND_ROOT /overlay
    (SOURCE). The tool will create an empty directory /overlay/etc/dir
    (if not available) and bind mount /overlay/etc/dir in /etc/dir. 

    Bind mount /etc/file (TARGET) file having the BIND_ROOT /overlay
    (SOURCE). The tool will create an empty file /overlay/etc/file (if
    not available) and bind mount /overlay/etc/file in /etc/file.

    In case of shadowing (TARGET directory/file is not empty) the tool
    warns and proceeds with mount.
```

***

# Support

If you're having any problem, please [raise an issue](https://github.com/balena-os/bindmount/issues/new)
on GitHub or [contact us](https://balena.io/community/), and the balena.io team will be happy to help.

***

# License

`bindmount` is free software, and may be redistributed under the terms specified in
the [license](https://github.com/balena-os/bindmount/blob/master/LICENSE).


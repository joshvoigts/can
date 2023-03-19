# can

Safer rm using AppleScript and Finder to move files to the trash
from the command line.

    usage: can [options] file ...
        -v, --verbose                    Run verbosely
        -l, --list                       List trash contents
        -E, --empty                      Empty trash
        -h, --help                       Show this message

## Release

    cargo build --release
    cd ~/builds/release/can
    tar -czf can-0.1.0-x86_64-apple-darwin.tar.gz can
    shasum -a 256 can-0.1.0-x86_64-apple-darwin.tar.gz

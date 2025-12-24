# can

Command line tool for moving files to the trash as an alternative
to `rm`.

## Install

### Cargo

    cargo install can-cli

### Homebrew

    brew tap joshvoigts/can
    brew install can

## Usage

    $ can --help
    usage: can [options] file ...
        -v, --verbose                    Run verbosely
        -l, --list                       List trash contents
        -E, --empty                      Empty trash
        -h, --help                       Show this message

### Release Instructions

    cargo build --release
    cd ~/builds/release
    tar -czf can-0.1.0-x86_64-apple-darwin.tar.gz can
    shasum -a 256 can-0.1.0-x86_64-apple-darwin.tar.gz

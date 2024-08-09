build:
    # only build VST3 and CLAP with `--lib`
    cargo xtask bundle malt --lib

release:
    # only build VST3 and CLAP with `--lib`
    cargo xtask bundle malt --lib --release

build:
    # only build VST3 and CLAP with `--lib`
    cargo xtask bundle sai_sampler --lib

release:
    # only build VST3 and CLAP with `--lib`
    cargo xtask bundle sai_sampler --lib --release

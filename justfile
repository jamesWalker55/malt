build:
    # only build VST3 and CLAP with `--lib`
    cargo xtask bundle malt --lib

release:
    # only build VST3 and CLAP with `--lib`
    cargo xtask bundle malt --lib --release

REAPER_PATH := "C:/Programs/REAPER Portable/reaper.exe"
reaper:
    # launch reaper in new instance
    # add an '&' or else bash just hangs forever
    "{{REAPER_PATH}}" -newinst &

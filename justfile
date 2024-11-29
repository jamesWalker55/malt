build:
    # only build VST3 and CLAP with `--lib`
    cargo xtask bundle malt --lib

release:
    # only build VST3 and CLAP with `--lib`
    cargo xtask bundle malt --lib --release

build-all:
    cargo xtask bundle malt

release-all:
    cargo xtask bundle malt --release

run:
    ./target/bundled/SAIAudio_Malt.exe

REAPER_PATH := "C:/Programs/REAPER Portable/reaper.exe"
reaper:
    # launch reaper in new instance
    # add an '&' or else bash just hangs forever
    "{{REAPER_PATH}}" -newinst &

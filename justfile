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
    # bash just hangs forever and also makes arrow keys type '[A' '[D' and some shit so we use powershell because fuck bash
    powershell -command ".\"{{REAPER_PATH}}\" -newinst"

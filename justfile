build:
    cargo xtask bundle malt

release:
    cargo xtask bundle malt --release

run:
    ./target/bundled/SAIAudio_Malt.exe

flamegraph:
    powershell -Command 'Start-Process powershell -Verb runAs -ArgumentList "-Command","cd `"{{invocation_directory()}}`" ; echo (Get-Item .).FullName ; cargo flamegraph --bin malt ; pause"'

REAPER_PATH := "C:/Programs/REAPER Portable/reaper.exe"
reaper:
    # launch reaper in new instance
    # bash just hangs forever and also makes arrow keys type '[A' '[D' and some shit so we use powershell because fuck bash
    powershell -Command ".\"{{REAPER_PATH}}\" -newinst"

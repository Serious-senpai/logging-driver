@echo off

for %%f in ("%~dp0..") do set root=%%~ff
echo Got root of repository: %root%

set last_cd=%cd%

cd /d %root%
cargo build --release

cd /d %root%\kernel
cargo make default --release

cd /d %last_cd%

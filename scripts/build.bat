@echo off

for %%f in ("%~dp0..") do set root=%%~ff
echo Got root of repository: %root%

set last_cd=%cd%
call :main
set exit_code=%errorlevel%

cd /d %last_cd%
exit /b %exit_code%

:main
cd /d %root%
cargo build --release || exit /b 1

cd /d %root%\kernel
cargo make default --release || exit /b 1

exit /b 0

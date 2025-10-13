@echo off

for %%f in ("%~dp0..") do set root=%%~ff
echo Got root of repository: %root%

if not exist "%root%\.vscode\" (
    mkdir "%root%\.vscode\"
)

echo @echo off> "%root%\.vscode\__run.bat"
echo cd /d %root%>> "%root%\.vscode\__run.bat"
echo set LIBCLANG_PATH=%root%\extern\clang-llvm-21.1.3>> "%root%\.vscode\__run.bat"
echo cmd>> "%root%\.vscode\__run.bat"

copy /-y "%root%\.vscode\__run.bat" "%root%\run.bat"
copy /-y "%root%\vscode.json" "%root%\.vscode\settings.json"

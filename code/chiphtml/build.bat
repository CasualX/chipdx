@echo off
setlocal

set "SCRIPT_DIR=%~dp0"
for %%I in ("%SCRIPT_DIR%..\..") do set "REPO_ROOT=%%~fI"

pushd "%REPO_ROOT%" || exit /b 1

cargo build --release -p chipwasm --target=wasm32-unknown-unknown || goto :fail

copy /Y "target\wasm32-unknown-unknown\release\chipwasm.wasm" "code\chiphtml\public\chipwasm.wasm" >nul || goto :fail
copy /Y "docs\social-preview.png" "code\chiphtml\public\social-preview.png" >nul || goto :fail

popd
exit /b 0

:fail
set "EXIT_CODE=%ERRORLEVEL%"
popd
exit /b %EXIT_CODE%

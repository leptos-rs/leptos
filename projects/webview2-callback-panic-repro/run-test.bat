@echo off
set NO_COLOR=
set RUSTFLAGS=
cd /d "%~dp0"

echo === leptos-webview2-repro autonomous test ===

echo.
echo --- Building frontend ---
cd frontend
cargo build --release --target wasm32-unknown-unknown --config "target.wasm32-unknown-unknown.rustflags=[]"
if errorlevel 1 (echo CARGO BUILD FAILED & exit /b 1)
trunk build --release
if errorlevel 1 (echo TRUNK BUILD FAILED & exit /b 1)
cd ..

echo.
echo --- Starting trunk serve (background) ---
start /B cmd /c "cd /d %~dp0frontend && set RUSTFLAGS= && trunk serve"
timeout /t 12 /nobreak >nul

echo.
echo --- Running Tauri app (60s) ---
cargo tauri dev
timeout /t 60 /nobreak >nul

echo.
echo === Test complete. Check window for panic (Ctrl+Shift+I for DevTools) ===

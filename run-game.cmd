@echo off
cd /d "%~dp0"
cargo run --offline -- %*
if errorlevel 1 pause

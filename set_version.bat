@echo off
setlocal enabledelayedexpansion

echo.
echo ==================================================
echo  EVE Abyssal Tracker 버전 자동 증가 스크립트
echo ==================================================
echo.

set "TAURI_CONF_PATH=eve-abyssal-tracker-tauri\src-tauri\tauri.conf.json"

echo 1. 현재 버전 확인 중...

:: PowerShell로 현재 버전 읽기
for /f "delims=" %%i in ('powershell -Command "(Get-Content '%TAURI_CONF_PATH%' | ConvertFrom-Json).version"') do set "CURRENT_VERSION=%%i"

if "%CURRENT_VERSION%"=="" (
    echo 오류: 현재 버전을 읽을 수 없습니다.
    goto :eof
)

echo 현재 버전: %CURRENT_VERSION%

:: 버전을 점으로 분리하여 각 부분 추출
for /f "tokens=1,2,3 delims=." %%a in ("%CURRENT_VERSION%") do (
    set "MAJOR=%%a"
    set "MINOR=%%b"
    set "PATCH=%%c"
)

:: 패치 버전 1 증가
set /a "NEW_PATCH=%PATCH%+1"
set "NEW_VERSION=%MAJOR%.%MINOR%.%NEW_PATCH%"

echo 새로운 버전: %NEW_VERSION%

echo.
echo 2. %TAURI_CONF_PATH% 파일 업데이트 중...
powershell -Command "(Get-Content '%TAURI_CONF_PATH%') -replace '\"version\": \"%CURRENT_VERSION%\"', '\"version\": \"%NEW_VERSION%\"' | Set-Content '%TAURI_CONF_PATH%'"

if %errorlevel% neq 0 (
    echo 오류: %TAURI_CONF_PATH% 파일 업데이트 실패.
    goto :eof
)
echo %TAURI_CONF_PATH% 업데이트 완료.

echo.
echo 3. Git 태그 생성 및 커밋 중...
set "GIT_TAG=v%NEW_VERSION%"
echo Git 태그: %GIT_TAG%

git add .
git commit -m "chore: Update version to %NEW_VERSION%"
git tag %GIT_TAG%

if %errorlevel% neq 0 (
    echo 오류: Git 태그 생성 실패.
    echo 수동으로 다음 명령어를 실행하여 문제를 해결하세요:
    echo   git add .
    echo   git commit -m "chore: Update version to %NEW_VERSION%"
    echo   git tag %GIT_TAG%
    goto :eof
)
echo Git 태그 생성 완료.

echo.
echo ==================================================
echo  버전 업데이트 완료!
echo  %CURRENT_VERSION% → %NEW_VERSION%
echo ==================================================
echo.

endlocal
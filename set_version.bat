@echo off
setlocal

set "NEW_VERSION=%1"

if "%NEW_VERSION%"=="" (
    echo 사용법: %~nx0 ^<새로운_버전^>
    echo 예시: %~nx0 1.0.23
    goto :eof
)

echo.
echo ==================================================
echo  EVE Abyssal Tracker 버전 업데이트 스크립트
echo ==================================================
echo.
echo 새로운 버전: %NEW_VERSION%

set "TAURI_CONF_PATH=eve-abyssal-tracker-tauri\src-tauri\tauri.conf.json"

echo.
echo 1. %TAURI_CONF_PATH% 파일 업데이트 중...
powershell -Command "(Get-Content '%TAURI_CONF_PATH%') -replace '\"version\": \"[0-9]+\.[0-9]+\.[0-9]+\"', '\"version\": \"%NEW_VERSION%\"' | Set-Content '%TAURI_CONF_PATH%'"

if %errorlevel% neq 0 (
    echo 오류: %TAURI_CONF_PATH% 파일 업데이트 실패.
    goto :eof
)
echo %TAURI_CONF_PATH% 업데이트 완료.

echo.
echo 2. Git 태그 생성 및 푸시 중...
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
echo ==================================================
echo.

endlocal
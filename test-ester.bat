@echo off
echo [KILL_PREV]
taskkill /F /IM ester_test.exe 2>nul

echo [LAUNCH]
start /b "" C:\Users\grunt\Desktop\ester_test.exe
echo launched

echo [WAIT_2s]
ping -n 3 127.0.0.1 > nul
tasklist | findstr /i ester_test || echo NOT_FOUND_2s

echo [WAIT_7s]
ping -n 6 127.0.0.1 > nul
tasklist | findstr /i ester_test || echo NOT_FOUND_7s

echo [WAIT_15s]
ping -n 9 127.0.0.1 > nul
tasklist | findstr /i ester_test || echo NOT_FOUND_15s

echo [WAIT_30s]
ping -n 16 127.0.0.1 > nul
tasklist | findstr /i ester_test || echo NOT_FOUND_30s

echo [DONE]

import os
import time
import threading
import sys
import queue # Add queue for inter-thread communication
import traceback # For detailed error logging
from datetime import datetime, timedelta
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler
import tkinter as tk
from pystray import Icon, Menu, MenuItem
from PIL import Image # pystray 아이콘에 필요
import webbrowser # Add webbrowser for opening Streamlit app

from .config_manager import ConfigManager
from .eve_log import EveLogProcessor
from ..ui.ui_popup import AbyssalResultPopup
from .global_abyssal_data_manager import GlobalAbyssalDataManager

class AbyssalRunTracker:
    def __init__(self, config_manager: ConfigManager, log_processor: EveLogProcessor,
                 popup_manager: AbyssalResultPopup, global_data_manager: GlobalAbyssalDataManager):
        self.config_manager = config_manager
        self.log_processor = log_processor
        self.popup_manager = popup_manager
        self.global_data_manager = global_data_manager

        self.logs_path = self.config_manager.get_logs_path()
        self.character_name = self.config_manager.get_character_name()
        self.language = self.config_manager.get_language()

        self.log_file = None
        self.last_position = 0
        self.abyssal_run_start = None
        self.abyssal_run_start_kst = None
        self.abyssal_run_count = 0
        self.current_system = None
        self.monitoring = False
        self.observer = None
        self.lock = threading.Lock()
        self.runs_by_date = {}
        self.root = None # Tkinter main window
        self.icon = None # pystray icon
        self.popup_queue = queue.Queue() # Queue for popup requests

        # Initialize log_processor with logs_path and language
        self.log_processor.logs_path = self.logs_path
        if self.language:
            self.log_processor.language = self.language

    def _find_latest_local_log(self):
        files = self.log_processor.find_all_log_files()
        if not files:
            print(f"[INFO] No local chat log files found in '{self.logs_path}'.")
            return None
        files.sort(key=os.path.getmtime, reverse=True)
        latest_file = files[0]
        
        self.log_processor.set_log_file(latest_file)
        detected_name = self.log_processor.detect_character_name(latest_file)
        
        if detected_name and not self.character_name: # Only auto-detect if not set in config
            self.character_name = detected_name
            print(f"[INFO] Auto-detected character name from latest log: '{self.character_name}' (File: {os.path.basename(latest_file)}, lang: {self.log_processor.language})")
        elif self.character_name and detected_name and self.character_name != detected_name:
            print(f"[WARNING] Configured character name '{self.character_name}' does not match detected name '{detected_name}' from log file '{os.path.basename(latest_file)}'. Using configured name.")
        elif not self.character_name:
            print(f"[WARNING] Could not auto-detect character name from the latest log file: '{os.path.basename(latest_file)}'. Monitoring might not start correctly.")
        
        return latest_file

    def _process_new_lines(self):
        if not self.log_file or not self.log_processor.patterns:
            return
        with self.lock:
            try:
                with open(self.log_file, "r", encoding="utf-16-le", errors='ignore') as f:
                    f.seek(self.last_position)
                    lines = f.readlines()
                    self.last_position = f.tell()
            except Exception as e:
                print(f"[ERROR] Failed to read log file '{self.log_file}': {e}")
                return
        for line in lines:
            self._on_system_change(line.strip().lstrip('\ufeff'))

    def _on_system_change(self, line):
        if self.log_processor.is_system_change_line(line):
            system_name, ts = self.log_processor.parse_system_change(line)
            if not system_name or not ts:
                return
            try:
                event_time = datetime.strptime(ts, "%Y.%m.%d %H:%M:%S")
                event_time_kst = event_time + timedelta(hours=9)
            except Exception:
                return
            if self.log_processor.is_unknown_system(system_name):
                if not self.abyssal_run_start:
                    self.abyssal_run_start = event_time
                    self.abyssal_run_start_kst = event_time_kst
                    print(f"[START] Abyssal Deadspace entered at {self.abyssal_run_start_kst.strftime('%H:%M:%S')} (KST)")
            else:
                if self.abyssal_run_start:
                    end_time = event_time
                    end_time_kst = event_time_kst
                    duration = end_time - self.abyssal_run_start
                    mins, secs = divmod(duration.seconds, 60)
                    self.abyssal_run_count += 1
                    
                    print(f"[END] Returned to normal space at {end_time_kst.strftime('%H:%M:%S')} (KST). Run duration: {mins}m {secs}s. Total runs: {self.abyssal_run_count}")
                    # 팝업 요청을 큐에 추가
                    self.popup_queue.put({
                        "type": "abyssal_result",
                        "start_time": self.abyssal_run_start_kst,
                        "end_time": end_time_kst
                    })
                    self.abyssal_run_start = None
                    self.abyssal_run_start_kst = None
            self.current_system = system_name

    def _on_log_folder_change(self, event):
        latest = self._find_latest_local_log()
        if latest and latest != self.log_file:
            print(f"[INFO] Switching to new log file: {os.path.basename(latest)}")
            self.log_file = latest
            self.last_position = os.path.getsize(self.log_file)
        elif not latest and self.log_file:
            print(f"[WARNING] Current log file '{os.path.basename(self.log_file)}' is no longer the preferred log. No suitable new log found.")
            self.log_file = None
            self.last_position = 0

    def _monitor_loop(self):
        while self.monitoring:
            if not self.log_file:
                self.log_file = self._find_latest_local_log()
                if self.log_file:
                    print(f"[INFO] Monitoring log file: {os.path.basename(self.log_file)} (full path: {self.log_file})")
                    self.last_position = os.path.getsize(self.log_file)
                else:
                    print("[INFO] Waiting for a suitable log file...")
                    time.sleep(5)
                    continue

            self._process_new_lines()
            time.sleep(2)

    def _scan_past_runs(self):
        files = self.log_processor.find_all_log_files()
        files = sorted(files, key=os.path.getmtime)
        print(f"[INFO] Scanning {len(files)} past log files for runs...")
        for file in files:
            # Create a temporary log processor for scanning past files
            temp_log_processor = EveLogProcessor(self.logs_path, self.language)
            temp_log_processor.set_log_file(file)

            if self.character_name:
                detected_name = temp_log_processor.detect_character_name(file)
                if not detected_name or detected_name != self.character_name:
                    continue
            try:
                abyssal_run_start = None
                for line in temp_log_processor.iter_lines(file):
                    if temp_log_processor.is_system_change_line(line):
                        system_name, ts = temp_log_processor.parse_system_change(line)
                        if not system_name or not ts:
                            continue
                        try:
                            event_time = datetime.strptime(ts, "%Y.%m.%d %H:%M:%S")
                        except Exception:
                            continue
                        if temp_log_processor.is_unknown_system(system_name):
                            if not abyssal_run_start:
                                try:
                                    abyssal_run_start = datetime.strptime(ts, "%Y.%m.%d %H:%M:%S")
                                except Exception:
                                    abyssal_run_start = None
                        else:
                            if abyssal_run_start:
                                try:
                                    end_time = datetime.strptime(ts, "%Y.%m.%d %H:%M:%S")
                                except Exception:
                                    end_time = None
                                if end_time:
                                    duration = end_time - abyssal_run_start
                                    mins, secs = divmod(duration.seconds, 60)
                                    date_str = abyssal_run_start.strftime('%Y-%m-%d')
                                    run_data = {
                                        'start': abyssal_run_start,
                                        'end': end_time,
                                        'duration': duration,
                                        'duration_str': f"{mins}m {secs}s"
                                    }
                                    if date_str not in self.runs_by_date:
                                        self.runs_by_date[date_str] = []
                                    self.runs_by_date[date_str].append(run_data)
                                abyssal_run_start = None
            except Exception as e:
                print(f"[ERROR] Failed to scan log file {file}: {e}")

    def print_past_runs(self):
        if self.runs_by_date:
            print("[PAST RUNS]")
            for date, runs in sorted(self.runs_by_date.items()):
                print(f"{date}:")
                for run in runs:
                    start_kst = run['start'] + timedelta(hours=9)
                    end_kst = run['end'] + timedelta(hours=9)
                    start_str = start_kst.strftime('%H:%M:%S')
                    end_str = end_kst.strftime('%H:%M:%S')
                    print(f"  - {start_str} ~ {end_str} ({run['duration_str']}) (KST)")
        else:
            print("[PAST RUNS] None")

    def _show_stats_from_tray(self, icon, item):
        print("[INFO] Streamlit 통계 페이지를 브라우저에서 엽니다.")
        # Streamlit 앱이 실행 중인지 확인하는 로직 (간단한 포트 체크)
        webbrowser.open("http://localhost:8501")

    def _exit_application(self, icon, item):
        print("[INFO] AbyssalTracker is shutting down.")
        self.monitoring = False
        if self.observer:
            self.observer.stop()
            self.observer.join()
        if self.root:
            self.root.quit() # Tkinter mainloop 종료
        icon.stop() # pystray 아이콘 종료

    def _check_popup_queue(self):
        try:
            while not self.popup_queue.empty():
                request = self.popup_queue.get_nowait()
                print(f"[DEBUG] Processing popup queue request: {request['type']}")
                if request["type"] == "abyssal_result":
                    start_time = request["start_time"]
                    end_time = request["end_time"]
                    try:
                        result = self.popup_manager.show_popup()
                        # GlobalAbyssalDataManager의 save_abyssal_result를 호출
                        self.global_data_manager.save_abyssal_result(result, start_time, end_time)
                        print(f"[INFO] Run result saved to CSV via GlobalAbyssalDataManager.")
                    except Exception as e:
                        print(f"[ERROR] UI/CSV 저장 중 오류: {e}")
                        traceback.print_exc() # 스택 트레이스 출력
                elif request["type"] == "info_message":
                    title = request.get("title", "알림")
                    message = request.get("message", "")
                    self.popup_manager.show_info_message(title, message)
        except queue.Empty:
            pass # No items in queue
        finally:
            self.root.after(100, self._check_popup_queue) # 다음 확인 예약

    def start_monitoring(self):
        self.monitoring = True
        
        # Tkinter 메인 윈도우 초기화 및 숨기기
        self.root = tk.Tk()
        self.root.withdraw() # 메인 윈도우 숨기기
        self.popup_manager.set_parent_root(self.root) # 팝업 매니저에 메인 Tkinter 루트 전달

        self.log_file = self._find_latest_local_log()
        if self.log_file:
            self.last_position = os.path.getsize(self.log_file)

        if self.log_file and self.character_name:
            print(f"[INFO] AbyssalTracker started for character: {self.character_name}")
            self._scan_past_runs()

            class LogFolderHandler(FileSystemEventHandler):
                def __init__(self, tracker):
                    self.tracker = tracker
                def on_created(self, event):
                    if not event.is_directory:
                        self.tracker._on_log_folder_change(event)
                def on_moved(self, event):
                    if not event.is_directory:
                        self.tracker._on_log_folder_change(event)

            self.observer = Observer()
            self.observer.schedule(LogFolderHandler(self), self.logs_path, recursive=False)
            self.observer.start()

            # 로그 모니터링을 별도의 스레드에서 실행
            monitor_thread = threading.Thread(target=self._monitor_loop)
            monitor_thread.daemon = True # 메인 프로그램 종료 시 스레드도 종료
            monitor_thread.start()

            # 시스템 트레이 아이콘 설정
            # 아이콘 이미지를 생성하거나, 실제 .ico 파일을 로드할 수 있습니다.
            # 여기서는 간단한 투명 이미지를 사용합니다. 실제 배포 시에는 적절한 아이콘 파일을 사용하세요.
            # 예: Image.open("path/to/your/icon.ico")
            icon_image = Image.new('RGBA', (64, 64), (0, 0, 0, 0)) # 투명 이미지
            # 또는 실제 아이콘 파일 로드 (예: icon.ico 파일이 프로젝트 루트에 있다고 가정)
            # try:
            #     icon_image = Image.open("icon.ico")
            # except FileNotFoundError:
            #     print("[WARNING] icon.ico not found. Using a transparent default icon.")
            #     icon_image = Image.new('RGBA', (64, 64), (0, 0, 0, 0))

            self.icon = Icon(
                'abyssal_tracker',
                icon_image,
                'Abyssal Tracker',
                menu=Menu(
                    MenuItem('통계 보기', self._show_stats_from_tray),
                    MenuItem('종료', self._exit_application)
                )
            )
            # Tkinter 메인 루프에서 주기적으로 팝업 큐 확인
            self.root.after(100, self._check_popup_queue) # 100ms마다 팝업 큐 확인

            # pystray 아이콘을 별도의 스레드에서 실행
            icon_thread = threading.Thread(target=self.icon.run)
            icon_thread.daemon = True
            icon_thread.start()

            print("[INFO] AbyssalTracker running in system tray.")
            self.root.mainloop() # Tkinter 메인 루프 시작

        else:
            print("[ERROR] Failed to initialize AbyssalTracker. No suitable log file found or character name not detected.")
            self.monitoring = False
            if self.root:
                self.root.destroy() # Tkinter 루트 윈도우 정리

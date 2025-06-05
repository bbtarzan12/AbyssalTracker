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

from config_manager import ConfigManager
from eve_log import EveLogProcessor
from ui_popup import AbyssalResultPopup
from price import AbyssalDataManager, EVEApi, AbyssalPriceAnalyzer
from ui_stats_display import AbyssalStatsDisplayPopup

class AbyssalRunTracker:
    def __init__(self, config_manager: ConfigManager, log_processor: EveLogProcessor,
                 popup_manager: AbyssalResultPopup, data_manager: AbyssalDataManager):
        self.config_manager = config_manager
        self.log_processor = log_processor
        self.popup_manager = popup_manager
        self.data_manager = data_manager

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
        print("[DEBUG] _show_stats_from_tray called. Adding 'show_stats' to queue.")
        # 통계 분석 및 팝업 요청을 큐에 추가
        self.popup_queue.put({"type": "show_stats"})


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
                        self.popup_manager.save_result(start_time, end_time)
                        print(f"[INFO] Run result saved to CSV.")
                    except Exception as e:
                        print(f"[ERROR] UI/CSV 저장 중 오류: {e}")
                        traceback.print_exc() # 스택 트레이스 출력
                elif request["type"] == "show_stats":
                    print("[DEBUG] Handling 'show_stats' request.")
                    try:
                        # 팝업을 먼저 띄우고 로딩 메시지 표시
                        AbyssalStatsDisplayPopup.set_parent_root(self.root)
                        self.stats_popup = AbyssalStatsDisplayPopup() # 초기에는 데이터 없이 로딩 상태로 생성
                        print("[DEBUG] AbyssalStatsDisplayPopup initialized with loading state.")

                        # 통계 데이터 로딩을 별도의 스레드에서 비동기적으로 실행
                        def load_stats_async():
                            try:
                                from rich.console import Console
                                dummy_console = Console(file=open(os.devnull, 'w'))
                                
                                eve_api = EVEApi()
                                data_manager = AbyssalDataManager()
                                analyzer = AbyssalPriceAnalyzer(eve_api, data_manager, dummy_console)
                                
                                print("[DEBUG] Calling analyzer.get_analysis_data() in async thread...")
                                stats_data = analyzer.get_analysis_data()
                                print(f"[DEBUG] analyzer.get_analysis_data() returned in async thread: {bool(stats_data)}")

                                # 데이터 로딩 완료 후, 메인 UI 스레드로 업데이트 요청 전달
                                self.popup_queue.put({"type": "update_stats", "stats_data": stats_data})
                            except Exception as e:
                                print(f"[ERROR] 비동기 통계 데이터 로딩 중 오류: {e}")
                                traceback.print_exc()
                        
                        threading.Thread(target=load_stats_async, daemon=True).start()

                    except Exception as e:
                        print(f"[ERROR] 통계 팝업 초기화 중 오류: {e}")
                        traceback.print_exc() # 스택 트레이스 출력
                elif request["type"] == "update_stats":
                    print("[DEBUG] Handling 'update_stats' request.")
                    stats_data = request["stats_data"]
                    if hasattr(self, 'stats_popup') and self.stats_popup.winfo_exists():
                        if stats_data:
                            print("[DEBUG] Updating existing AbyssalStatsDisplayPopup with loaded data.")
                            self.stats_popup.update_content(stats_data)
                        else:
                            print("[INFO] 통계 데이터를 불러올 수 없습니다. CSV 파일이 비어있거나 오류가 있습니다.")
                            # 로딩 메시지를 유지하거나, 오류 메시지로 변경할 수 있습니다.
                            if hasattr(self.stats_popup, 'loading_label'):
                                self.stats_popup.loading_label.config(text="데이터 로딩 실패!")
                    else:
                        print("[WARNING] 'update_stats' 요청이 왔으나, 팝업 인스턴스가 존재하지 않습니다. 새로 생성합니다.")
                        AbyssalStatsDisplayPopup.set_parent_root(self.root)
                        self.stats_popup = AbyssalStatsDisplayPopup(stats_data) # 데이터와 함께 새로 생성
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

if __name__ == "__main__":
    # .pyc 파일 생성을 방지 (선택 사항)
    sys.dont_write_bytecode = True

    config_manager = ConfigManager()
    config_manager.validate_config() # Validate config before proceeding

    log_processor = EveLogProcessor(
        logs_path=config_manager.get_logs_path(),
        language=config_manager.get_language()
    )
    data_manager = AbyssalDataManager()
    popup_manager = AbyssalResultPopup(data_manager)

    tracker = AbyssalRunTracker(
        config_manager=config_manager,
        log_processor=log_processor,
        popup_manager=popup_manager,
        data_manager=data_manager
    )
    
    # 콘솔 창 없이 실행하려면 'pythonw.exe'를 사용해야 합니다.
    # 예: pythonw tracker.py
    tracker.start_monitoring()
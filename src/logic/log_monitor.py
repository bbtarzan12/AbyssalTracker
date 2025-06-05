import os
import time
import threading
import queue
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler
from typing import Optional, Callable

from src.logic.config_manager import ConfigManager
from src.logic.eve_log import EveLogProcessor

class LogMonitor:
    def __init__(self, config_manager: ConfigManager, log_processor: EveLogProcessor,
                 on_new_log_lines: Callable[[list[str]], None], on_log_file_change: Callable[[], None]):
        self.config_manager = config_manager
        self.log_processor = log_processor
        self.on_new_log_lines = on_new_log_lines
        self.on_log_file_change = on_log_file_change

        self.logs_path = self.config_manager.get_logs_path()
        self.character_name = self.config_manager.get_character_name()
        self.language = self.config_manager.get_language()

        self.log_file: Optional[str] = None
        self.last_position: int = 0
        self.monitoring: bool = False
        self.observer: Optional[Observer] = None
        self.lock = threading.Lock()

        self.log_processor.logs_path = self.logs_path
        if self.language:
            self.log_processor.language = self.language

    def _find_latest_local_log(self) -> Optional[str]:
        files = self.log_processor.find_all_log_files()
        if not files:
            print(f"[INFO] No local chat log files found in '{self.logs_path}'.")
            return None
        files.sort(key=os.path.getmtime, reverse=True)
        latest_file = files[0]
        
        self.log_processor.set_log_file(latest_file)
        detected_name = self.log_processor.detect_character_name(latest_file)
        
        if detected_name and not self.character_name:
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
        if lines:
            self.on_new_log_lines([line.strip().lstrip('\ufeff') for line in lines])

    def _on_log_folder_change_event(self, event):
        latest = self._find_latest_local_log()
        if latest and latest != self.log_file:
            print(f"[INFO] Switching to new log file: {os.path.basename(latest)}")
            self.log_file = latest
            self.last_position = os.path.getsize(self.log_file)
            self.on_log_file_change() # Notify tracker about file change
        elif not latest and self.log_file:
            print(f"[WARNING] Current log file '{os.path.basename(self.log_file)}' is no longer the preferred log. No suitable new log found.")
            self.log_file = None
            self.last_position = 0
            self.on_log_file_change() # Notify tracker about file change

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

    def start(self):
        self.monitoring = True
        self.log_file = self._find_latest_local_log()
        if self.log_file:
            self.last_position = os.path.getsize(self.log_file)

        if self.log_file and self.character_name:
            print(f"[INFO] LogMonitor started for character: {self.character_name}")

            class LogFolderHandler(FileSystemEventHandler):
                def __init__(self, monitor):
                    self.monitor = monitor
                def on_created(self, event):
                    if not event.is_directory:
                        self.monitor._on_log_folder_change_event(event)
                def on_moved(self, event):
                    if not event.is_directory:
                        self.monitor._on_log_folder_change_event(event)

            self.observer = Observer()
            self.observer.schedule(LogFolderHandler(self), self.logs_path, recursive=False)
            self.observer.start()

            monitor_thread = threading.Thread(target=self._monitor_loop)
            monitor_thread.daemon = True
            monitor_thread.start()
        else:
            print("[ERROR] LogMonitor failed to initialize. No suitable log file found or character name not detected.")
            self.monitoring = False

    def stop(self):
        self.monitoring = False
        if self.observer:
            self.observer.stop()
            self.observer.join()
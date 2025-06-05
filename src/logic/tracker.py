import os
import time
import threading
import sys
import queue
import traceback
from datetime import datetime, timedelta
from pystray import Icon, Menu, MenuItem
from PIL import Image
import webbrowser
import tkinter as tk # Tkinter 임포트 추가

from .config_manager import ConfigManager
from .eve_log import EveLogProcessor
from .abyssal_data_analyzer import AbyssalDataAnalyzer
from .log_monitor import LogMonitor # LogMonitor 임포트
from .system_change_processor import SystemChangeProcessor # SystemChangeProcessor 임포트
from ..ui.ui_popup import AbyssalResultPopup # AbyssalResultPopup 임포트
from ..ui.ui_notifier import UINotifier # UINotifier 임포트

class AbyssalRunTracker:
    def __init__(self, config_manager: ConfigManager, log_processor: EveLogProcessor,
                 popup_manager: AbyssalResultPopup, abyssal_data_analyzer: AbyssalDataAnalyzer):
        self.config_manager = config_manager
        self.log_processor = log_processor
        self.popup_manager = popup_manager # popup_manager 속성 다시 추가
        self.abyssal_data_analyzer = abyssal_data_analyzer

        self.logs_path = self.config_manager.get_logs_path()
        self.character_name = self.config_manager.get_character_name()
        self.language = self.config_manager.get_language()

        self.root: Optional[tk.Tk] = None
        self.icon: Optional[Icon] = None

        # Initialize sub-components
        self.log_monitor = LogMonitor(
            config_manager=self.config_manager,
            log_processor=self.log_processor,
            on_new_log_lines=self._on_new_log_lines,
            on_log_file_change=self._on_log_file_change
        )
        self.system_change_processor = SystemChangeProcessor(
            log_processor=self.log_processor,
            on_abyssal_run_end=self._on_abyssal_run_end
        )
        self.ui_notifier: Optional[UINotifier] = None # Will be initialized with self.root

    def _on_new_log_lines(self, lines: list[str]):
        for line in lines:
            self.system_change_processor.process_log_line(line)

    def _on_log_file_change(self):
        # Log file changed, potentially re-scan past runs or adjust state
        print("[INFO] Log file changed. Re-scanning past runs.")
        self.system_change_processor.scan_past_runs(self.logs_path, self.character_name)
        self.system_change_processor.print_past_runs()

    def _on_abyssal_run_end(self, start_time: datetime, end_time: datetime):
        # This callback is from SystemChangeProcessor when a run ends
        if self.ui_notifier:
            self.ui_notifier.put_popup_request({
                "type": "abyssal_result",
                "start_time": start_time,
                "end_time": end_time
            })

    def _show_stats_from_tray(self, icon, item):
        print("[INFO] Streamlit 통계 페이지를 브라우저에서 엽니다.")
        webbrowser.open("http://localhost:8501")

    def _exit_application(self, icon, item):
        print("[INFO] AbyssalTracker is shutting down.")
        self.log_monitor.stop() # Stop the log monitor
        if self.root:
            self.root.quit()
        icon.stop()

    def start_monitoring(self):
        self.root = tk.Tk()
        self.root.withdraw()

        self.ui_notifier = UINotifier(
            parent_root=self.root,
            popup_manager=self.popup_manager,
            abyssal_data_analyzer=self.abyssal_data_analyzer
        )
        self.ui_notifier.start_queue_monitoring()

        self.log_monitor.start() # Start the log monitor

        if self.log_monitor.log_file and self.character_name:
            print(f"[INFO] AbyssalTracker started for character: {self.character_name}")
            self.system_change_processor.scan_past_runs(self.logs_path, self.character_name)
            self.system_change_processor.print_past_runs()

            icon_image = Image.new('RGBA', (64, 64), (0, 0, 0, 0))
            self.icon = Icon(
                'abyssal_tracker',
                icon_image,
                'Abyssal Tracker',
                menu=Menu(
                    MenuItem('통계 보기', self._show_stats_from_tray),
                    MenuItem('종료', self._exit_application)
                )
            )
            icon_thread = threading.Thread(target=self.icon.run)
            icon_thread.daemon = True
            icon_thread.start()

            print("[INFO] AbyssalTracker running in system tray.")
            self.root.mainloop()
        else:
            print("[ERROR] Failed to initialize AbyssalTracker. No suitable log file found or character name not detected.")
            if self.root:
                self.root.destroy()

import tkinter as tk
import queue
import traceback
from typing import Any, Callable

from src.ui.ui_popup import AbyssalResultPopup
from src.logic.abyssal_data_analyzer import AbyssalDataAnalyzer # AbyssalDataAnalyzer 임포트

class UINotifier:
    def __init__(self, parent_root: tk.Tk, popup_manager: AbyssalResultPopup,
                 abyssal_data_analyzer: AbyssalDataAnalyzer):
        self.parent_root = parent_root
        self.popup_manager = popup_manager
        self.abyssal_data_analyzer = abyssal_data_analyzer
        self.popup_queue: queue.Queue = queue.Queue()

        self.popup_manager.set_parent_root(self.parent_root)

    def put_popup_request(self, request: dict):
        self.popup_queue.put(request)

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
                        self.abyssal_data_analyzer.data_manager.save_abyssal_result(result, start_time, end_time)
                        print(f"[INFO] Run result saved to CSV via AbyssalDataManager.")
                    except Exception as e:
                        print(f"[ERROR] UI/CSV 저장 중 오류: {e}")
                        traceback.print_exc()
                elif request["type"] == "info_message":
                    title = request.get("title", "알림")
                    message = request.get("message", "")
                    # Assuming AbyssalResultPopup has a show_info_message method
                    # If not, this would need to be implemented or handled differently
                    if hasattr(self.popup_manager, 'show_info_message'):
                        self.popup_manager.show_info_message(title, message)
                    else:
                        print(f"[WARNING] show_info_message not implemented in popup_manager. Title: {title}, Message: {message}")
        except queue.Empty:
            pass
        finally:
            self.parent_root.after(100, self._check_popup_queue)

    def start_queue_monitoring(self):
        self.parent_root.after(100, self._check_popup_queue)
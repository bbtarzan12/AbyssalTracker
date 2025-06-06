import os
from datetime import datetime, timedelta
from typing import Optional, Callable, Dict, Any

from src.logic.eve_log import EveLogProcessor

class SystemChangeProcessor:
    def __init__(self, log_processor: EveLogProcessor,
                 on_abyssal_run_end: Callable[[datetime, datetime], None]):
        self.log_processor = log_processor
        self.on_abyssal_run_end = on_abyssal_run_end
        
        self.abyssal_run_start: Optional[datetime] = None
        self.abyssal_run_start_kst: Optional[datetime] = None
        self.abyssal_run_count: int = 0
        self.current_system: Optional[str] = None
        self.runs_by_date: Dict[str, Any] = {} # For past runs scanning

    def process_log_line(self, line: str):
        if self.log_processor.is_system_change_line(line):
            system_name, ts = self.log_processor.parse_system_change(line)
            if not system_name or not ts:
                return
            try:
                event_time = datetime.strptime(ts, "%Y.%m.%d %H:%M:%S")
                event_time_kst = event_time + timedelta(hours=9)
            except ValueError:
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
                    self.on_abyssal_run_end(self.abyssal_run_start_kst, end_time_kst)
                    self.abyssal_run_start = None
                    self.abyssal_run_start_kst = None
            self.current_system = system_name

    def scan_past_runs(self, logs_path: str, character_name: str):
        files = self.log_processor.find_all_log_files()
        files = sorted(files, key=os.path.getmtime)
        print(f"[INFO] Scanning {len(files)} past log files for runs...")
        for file in files:
            temp_log_processor = EveLogProcessor(logs_path, self.log_processor.language)
            temp_log_processor.set_log_file(file)

            if character_name:
                detected_name = temp_log_processor.detect_character_name(file)
                if not detected_name or detected_name != character_name:
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
                        except ValueError:
                            continue
                        if temp_log_processor.is_unknown_system(system_name):
                            if not abyssal_run_start:
                                try:
                                    abyssal_run_start = datetime.strptime(ts, "%Y.%m.%d %H:%M:%S")
                                except ValueError:
                                    abyssal_run_start = None
                        else:
                            if abyssal_run_start:
                                try:
                                    end_time = datetime.strptime(ts, "%Y.%m.%d %H:%M:%S")
                                except ValueError:
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
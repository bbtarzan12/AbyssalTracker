import re
import glob
import os
from datetime import datetime, timedelta

SYSTEM_CHANGE_PATTERNS = {
    'ko': {
        'system_change': "이브 시스템 > 지역 : ",
        'channel_changed': "채널로 변경",
        'unknown_system': "알 수 없음"
    },
    'en': {
        'system_change': "EVE System > Channel changed to Local : ",
        'channel_changed': "",
        'unknown_system': "Unknown"
    },
    # 필요시 다른 언어 추가
}

LOG_FILENAME_PATTERNS = {
    'ko': '지역_*.txt',
    'en': 'Local_*.txt',
    # 필요시 다른 언어 추가
}

class EveLogProcessor:
    SYSTEM_CHANGE_PATTERNS = SYSTEM_CHANGE_PATTERNS
    LOG_FILENAME_PATTERNS = LOG_FILENAME_PATTERNS

    def __init__(self, logs_path, language=None):
        self.logs_path = logs_path
        self._current_log_file = None
        self.language = language # If None, will be detected later
        self.patterns = None # Will be set after language detection

    def find_all_log_files(self):
        files = []
        for pattern in self.LOG_FILENAME_PATTERNS.values():
            files.extend(glob.glob(os.path.join(self.logs_path, pattern)))
        return files

    def detect_log_language(self, file_path):
        try:
            with open(file_path, "r", encoding="utf-16-le", errors='ignore') as f:
                for _ in range(20):
                    line = f.readline()
                    for lang, patterns in self.SYSTEM_CHANGE_PATTERNS.items():
                        if patterns['system_change'] in line:
                            return lang
        except Exception:
            pass
        return 'ko'  # Default to Korean if detection fails

    def set_log_file(self, file_path):
        self._current_log_file = file_path
        if not self.language:
            self.language = self.detect_log_language(file_path)
        self.patterns = self.SYSTEM_CHANGE_PATTERNS[self.language]

    def iter_lines(self, file_path=None):
        target_file = file_path if file_path else self._current_log_file
        if not target_file:
            raise ValueError("No log file specified for iteration.")
        with open(target_file, "r", encoding="utf-16-le", errors='ignore') as f:
            for line in f:
                yield line.strip().lstrip('\ufeff')

    def iter_system_changes(self, file_path=None):
        for line in self.iter_lines(file_path):
            if self.is_system_change_line(line):
                system_name, ts = self.parse_system_change(line)
                if system_name and ts:
                    yield system_name, ts, line

    def is_system_change_line(self, line):
        return self.patterns['system_change'] in line and self.patterns['channel_changed'] in line

    def parse_system_change(self, line):
        try:
            system_name = line.split(self.patterns['system_change'])[1].split(self.patterns['channel_changed'])[0].strip()
            ts = line.split("]")[0].split("[")[1].strip()
            return system_name, ts
        except Exception:
            return None, None

    def is_unknown_system(self, system_name):
        return system_name == self.patterns['unknown_system']

    def detect_character_name(self, file_path=None):
        target_file = file_path if file_path else self._current_log_file
        if not target_file:
            return None
        try:
            with open(target_file, "r", encoding="utf-16-le", errors='ignore') as f:
                for i in range(20):
                    line = f.readline()
                    if not line:
                        break
                    processed_line = line.lstrip('\ufeff').strip()
                    match = re.search(r"Listener:\s*(.+)", processed_line)
                    if match:
                        return match.group(1).strip()
        except Exception:
            pass
        return None
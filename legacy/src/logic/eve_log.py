
from typing import Iterator, Tuple, Optional
import re
import glob
import os
from datetime import datetime, timedelta

from src.logic.constants import SYSTEM_CHANGE_PATTERNS, LOG_FILENAME_PATTERNS

class EveLogProcessor:
    def __init__(self, logs_path: str, language: Optional[str] = None):
        self.logs_path = logs_path
        self._current_log_file: Optional[str] = None
        self.language = language
        self.patterns: dict[str, str] = {} # Will be set after language detection or explicitly

    def find_all_log_files(self) -> list[str]:
        files = []
        for pattern in LOG_FILENAME_PATTERNS.values():
            files.extend(glob.glob(os.path.join(self.logs_path, pattern)))
        return files

    def detect_log_language(self, file_path: str) -> str:
        try:
            with open(file_path, "r", encoding="utf-16-le", errors='ignore') as f:
                for _ in range(20): # Read first 20 lines to detect language
                    line = f.readline()
                    if not line:
                        break
                    for lang, patterns in SYSTEM_CHANGE_PATTERNS.items():
                        if patterns['system_change'] in line:
                            return lang
        except (FileNotFoundError, UnicodeDecodeError):
            pass
        return 'ko'  # Default to Korean if detection fails

    def set_log_file(self, file_path: str):
        self._current_log_file = file_path
        if not self.language:
            self.language = self.detect_log_language(file_path)
        self.patterns = SYSTEM_CHANGE_PATTERNS.get(self.language, SYSTEM_CHANGE_PATTERNS['ko']) # Fallback to 'ko'

    def iter_lines(self, file_path: Optional[str] = None) -> Iterator[str]:
        target_file = file_path if file_path else self._current_log_file
        if not target_file:
            raise ValueError("No log file specified for iteration.")
        with open(target_file, "r", encoding="utf-16-le", errors='ignore') as f:
            for line in f:
                yield line.strip().lstrip('\ufeff')

    def iter_system_changes(self, file_path: Optional[str] = None) -> Iterator[Tuple[str, str, str]]:
        for line in self.iter_lines(file_path):
            if self.is_system_change_line(line):
                system_name, ts = self.parse_system_change(line)
                if system_name and ts:
                    yield system_name, ts, line

    def is_system_change_line(self, line: str) -> bool:
        return self.patterns.get('system_change', '') in line and self.patterns.get('channel_changed', '') in line

    def parse_system_change(self, line: str) -> Tuple[Optional[str], Optional[str]]:
        try:
            system_change_prefix = self.patterns.get('system_change', '')
            channel_changed_suffix = self.patterns.get('channel_changed', '')

            if not system_change_prefix or not channel_changed_suffix:
                return None, None

            # Extract system name
            system_part = line.split(system_change_prefix)[1]
            system_name = system_part.split(channel_changed_suffix)[0].strip()

            # Extract timestamp
            ts_match = re.search(r"\[(\d{4}\.\d{2}\.\d{2} \d{2}:\d{2}:\d{2})\]", line)
            ts = ts_match.group(1).strip() if ts_match else None

            return system_name, ts
        except (IndexError, AttributeError): # AttributeError for ts_match.group(1) if ts_match is None
            return None, None

    def is_unknown_system(self, system_name: str) -> bool:
        return system_name == self.patterns.get('unknown_system', '')

    def detect_character_name(self, file_path: Optional[str] = None) -> Optional[str]:
        target_file = file_path if file_path else self._current_log_file
        if not target_file:
            return None
        try:
            with open(target_file, "r", encoding="utf-16-le", errors='ignore') as f:
                for i in range(20): # Read first 20 lines to detect character name
                    line = f.readline()
                    if not line:
                        break
                    processed_line = line.lstrip('\ufeff').strip()
                    match = re.search(r"Listener:\s*(.+)", processed_line)
                    if match:
                        return match.group(1).strip()
        except (FileNotFoundError, UnicodeDecodeError):
            pass
        return None
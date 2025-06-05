import configparser
import os
import getpass

class ConfigManager:
    def __init__(self, config_file: str = 'config.ini'):
        self.config_file = config_file
        self.config = configparser.ConfigParser()
        self._load_config()

    def _load_config(self):
        if not os.path.exists(self.config_file):
            self._create_default_config()
        self.config.read(self.config_file, encoding='utf-8')

    def _create_default_config(self):
        username = getpass.getuser()
        default_logs_path = f"C:/Users/{username}/Documents/EVE/logs/Chatlogs"
        if os.path.exists(default_logs_path):
            logs_path = default_logs_path
        else:
            logs_path = ''
        self.config['DEFAULT'] = {
            'logs_path': logs_path,
            'character_name': '',
            'language': ''
        }
        with open(self.config_file, 'w', encoding='utf-8') as f:
            self.config.write(f)
        print(f"[INFO] {self.config_file} 파일이 생성되었습니다.\nlogs_path와 character_name 값을 확인/수정한 후 프로그램을 다시 실행하세요.")
        exit(1)

    def get(self, section: str, option: str, fallback: str | None = None) -> str | None:
        return self.config.get(section, option, fallback=fallback)

    def get_logs_path(self) -> str:
        return self.get('DEFAULT', 'logs_path', '').strip()

    def get_character_name(self) -> str:
        return self.get('DEFAULT', 'character_name', '').strip()

    def get_language(self) -> str:
        return self.get('DEFAULT', 'language', '').strip()

    def validate_config(self):
        logs_path = self.get_logs_path()
        character_name = self.get_character_name()
        if not logs_path or not character_name:
            print(f"[ERROR] {self.config_file}에 logs_path와 character_name 값을 모두 입력해야 합니다. 파일을 수정한 후 다시 실행하세요.")
            exit(1)
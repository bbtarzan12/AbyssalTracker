# src/logic/constants.py

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
    # Add other languages as needed
}

LOG_FILENAME_PATTERNS = {
    'ko': '지역_*.txt',
    'en': 'Local_*.txt',
    # Add other languages as needed
}
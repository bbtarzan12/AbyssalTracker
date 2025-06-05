import sys
import os
import subprocess
import atexit
import time

# Add src to the Python path to allow absolute imports
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), 'src')))

from src.logic.config_manager import ConfigManager
from src.logic.eve_log import EveLogProcessor
from src.ui.ui_popup import AbyssalResultPopup
from src.logic.global_abyssal_data_manager import GlobalAbyssalDataManager # GlobalAbyssalDataManager 임포트
from src.logic.tracker import AbyssalRunTracker

streamlit_process = None

def start_streamlit():
    global streamlit_process
    print("Starting Streamlit server in background...")
    # Streamlit 서버를 백그라운드에서 실행하고 웹 브라우저를 자동으로 열지 않도록 합니다.
    # 현재 작업 디렉토리가 abyssal_v2라고 가정합니다.
    streamlit_process = subprocess.Popen(
        [sys.executable, "-m", "streamlit", "run", "streamlit_app.py", "--server.headless", "true"],
        stdout=sys.stdout,
        stderr=sys.stderr,
        cwd=os.path.dirname(os.path.abspath(__file__)) # Ensure cwd is the project root
    )
    print(f"Streamlit server started with PID: {streamlit_process.pid}")
    # Streamlit이 시작될 시간을 잠시 기다립니다.
    time.sleep(5)

def terminate_streamlit():
    global streamlit_process
    if streamlit_process and streamlit_process.poll() is None:
        print("Terminating Streamlit server...")
        streamlit_process.terminate()
        try:
            streamlit_process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            streamlit_process.kill()
        print("Streamlit server terminated.")

if __name__ == "__main__":
    atexit.register(terminate_streamlit)

    start_streamlit()

    config_manager = ConfigManager()
    config_manager.validate_config() # Validate config before proceeding

    log_processor = EveLogProcessor(
        logs_path=config_manager.get_logs_path(),
        language=config_manager.get_language()
    )
    global_data_manager = GlobalAbyssalDataManager() # GlobalAbyssalDataManager 인스턴스 생성
    popup_manager = AbyssalResultPopup(global_data_manager.data_manager) # 기존 data_manager 대신 global_data_manager의 data_manager 사용

    tracker = AbyssalRunTracker(
        config_manager=config_manager,
        log_processor=log_processor,
        popup_manager=popup_manager,
        global_data_manager=global_data_manager # global_data_manager 인스턴스 직접 전달
    )
    
    # AbyssalRunTracker는 시스템 트레이 아이콘을 시작하고 모니터링 루프를 실행합니다.
    # 이 함수는 블로킹될 수 있습니다.
    tracker.start_monitoring()
import sys
import os

# Add src to the Python path to allow absolute imports
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), 'src')))

from src.logic.config_manager import ConfigManager
from src.logic.eve_log import EveLogProcessor
from src.ui.ui_popup import AbyssalResultPopup
from src.logic.price import AbyssalDataManager
from src.logic.tracker import AbyssalRunTracker

if __name__ == "__main__":
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
    
    tracker.start_monitoring()
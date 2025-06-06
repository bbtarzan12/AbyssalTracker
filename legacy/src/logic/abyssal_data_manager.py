import os
import glob
import re
import pandas as pd
import datetime

class AbyssalDataManager:
    def __init__(self, data_dir='data'):
        self.data_dir = data_dir

    def parse_items(self, item_str):
        items = []
        for entry in str(item_str).split(';'):
            entry = entry.strip()
            if not entry:
                continue
            m = re.match(r'(.+?)\*\s*(\d+)?$', entry)
            if m:
                name = m.group(1).strip()
                try:
                    qty = int(m.group(2)) if m.group(2) else 1
                except ValueError:
                    qty = 1
                items.append((name, qty))
            else:
                items.append((entry, 1))
        return items

    def abyssal_type_to_filament_name(self, abyssal_type: str) -> str | None:
        tier_map = {
            'T1': 'Calm', 'T2': 'Agitated', 'T3': 'Fierce',
            'T4': 'Raging', 'T5': 'Chaotic', 'T6': 'Cataclysmic',
        }
        try:
            tier, weather = abyssal_type.split()
            return f'{tier_map[tier]} {weather} Filament'
        except (ValueError, KeyError):
            return None

    def load_abyssal_results(self) -> pd.DataFrame:
        result_files = sorted(glob.glob(os.path.join(self.data_dir, 'abyssal_results_*.csv')))
        if not result_files:
            return pd.DataFrame()

        dfs = []
        for file in result_files:
            df = pd.read_csv(file, encoding='utf-8')
            dfs.append(df)
        return pd.concat(dfs, ignore_index=True)

    def save_abyssal_result(self, result: dict, start_time: datetime, end_time: datetime):
        items = result.get("items", "").strip()
        if not items:
            return

        os.makedirs(self.data_dir, exist_ok=True)
        date_str = start_time.strftime('%Y-%m-%d')
        filename = os.path.join(self.data_dir, f"abyssal_results_{date_str}.csv")
        file_exists = os.path.exists(filename)
        duration_sec = int((end_time - start_time).total_seconds())
        duration_min = round(duration_sec / 60, 2)
        items = items.replace('\n', '; ').replace('\r', '')
        row = {
            "시작시각(KST)": start_time.strftime('%Y-%m-%d %H:%M:%S'),
            "종료시각(KST)": end_time.strftime('%Y-%m-%d %H:%M:%S'),
            "런 소요(초)": duration_sec,
            "런 소요(분)": duration_min,
            "어비셜 종류": result.get("abyssal_type", ""),
            "획득 아이템": items
        }
        df = pd.DataFrame([row])
        df.to_csv(
            filename,
            mode='a',
            header=not file_exists,
            index=False,
            encoding='utf-8-sig'
        )
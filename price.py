import os
import glob
import re
import requests
import pandas as pd
from rich.console import Console
from rich.table import Table
import time
import json

class EVEApi:
    REGION_ID = 10000002  # The Forge
    STATION_ID = 60003760 # Jita 4-4
    CACHE_FILE = os.path.join(os.path.dirname(__file__), 'typeid_cache.json')

    def __init__(self):
        self.name_to_id_cache = self._load_cache()

    def _load_cache(self):
        try:
            with open(self.CACHE_FILE, 'r', encoding='utf-8') as f:
                return json.load(f)
        except Exception:
            return {}

    def _save_cache(self):
        with open(self.CACHE_FILE, 'w', encoding='utf-8') as f:
            json.dump(self.name_to_id_cache, f, ensure_ascii=False, indent=2)

    def fetch_type_ids(self, names):
        name_to_id = {}
        names_to_query = [n for n in names if n not in self.name_to_id_cache]

        for i in range(0, len(names_to_query), 20):
            chunk = names_to_query[i:i+20]
            try:
                resp = requests.post(
                    "https://esi.evetech.net/latest/universe/ids/",
                    json=chunk,
                    timeout=10,
                    headers={"User-Agent": "bulk-lookup 1.0", "Content-Type": "application/json"}
                )
                resp.raise_for_status()
                data = resp.json()
                for itm in data.get("inventory_types", []):
                    self.name_to_id_cache[itm["name"]] = itm["id"]
                time.sleep(0.2)
            except requests.exceptions.RequestException as e:
                print(f"[ERROR] ESI API 호출 중 오류 발생: {e}")
                continue
        
        self._save_cache()
        for n in names:
            if n in self.name_to_id_cache:
                name_to_id[n] = self.name_to_id_cache[n]
        return name_to_id

    def fetch_fuzzwork_prices(self, ids):
        url = (f"https://market.fuzzwork.co.uk/aggregates/?region={self.REGION_ID}"
               f"&types={','.join(map(str, ids))}")
        try:
            return requests.get(url, timeout=15).json()
        except requests.exceptions.RequestException as e:
            print(f"[ERROR] Fuzzwork API 호출 중 오류 발생: {e}")
            return {}

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
                except Exception:
                    qty = 1
                items.append((name, qty))
            else:
                items.append((entry, 1))
        return items

    def abyssal_type_to_filament_name(self, abyssal_type):
        tier_map = {
            'T1': 'Calm', 'T2': 'Agitated', 'T3': 'Fierce',
            'T4': 'Raging', 'T5': 'Chaotic', 'T6': 'Cataclysmic',
        }
        try:
            tier, weather = abyssal_type.split()
            return f'{tier_map[tier]} {weather} Filament'
        except Exception:
            return None

    def load_abyssal_results(self):
        result_files = sorted(glob.glob(os.path.join(self.data_dir, 'abyssal_results_*.csv')))
        if not result_files:
            return pd.DataFrame()

        dfs = []
        for file in result_files:
            df = pd.read_csv(file, encoding='utf-8')
            dfs.append(df)
        return pd.concat(dfs, ignore_index=True)

    def save_abyssal_result(self, result, start_time, end_time):
        items = result.get("items", "").strip()
        if not items:
            return  # 아이템이 비어있으면 기록하지 않음

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

class AbyssalPriceAnalyzer:
    def __init__(self, eve_api: EVEApi, data_manager: AbyssalDataManager, console: Console):
        self.eve_api = eve_api
        self.data_manager = data_manager
        self.console = console
        self.item_buy_price_cache = {}
        self.item_sell_price_cache = {}
        self.df = pd.DataFrame()

    def get_analysis_data(self):
        # 1. CSV 파일 읽기
        self.df = self.data_manager.load_abyssal_results()
        if self.df.empty:
            return None # 분석할 데이터가 없으면 None 반환

        # 2. 모든 아이템 이름 수집
        all_item_names = set()
        for items in self.df['획득 아이템']:
            for name, qty in self.data_manager.parse_items(items):
                all_item_names.add(name)
        for abyssal_type in self.df['어비셜 종류']:
            filament = self.data_manager.abyssal_type_to_filament_name(abyssal_type)
            if filament:
                all_item_names.add(filament)
        all_item_names = list(all_item_names)

        # 3. ESI API로 아이템 type_id 변환
        name_to_id = self.eve_api.fetch_type_ids(all_item_names)

        # 4. Fuzzwork로 대량 시세 조회
        ids = list(name_to_id.values())
        prices = {}
        for i in range(0, len(ids), 100):
            chunk = ids[i:i+100]
            prices.update(self.eve_api.fetch_fuzzwork_prices(chunk))
            time.sleep(0.2)

        for name, type_id in name_to_id.items():
            p = prices.get(str(type_id))
            if p:
                try:
                    self.item_buy_price_cache[name] = float(p['buy']['max'])
                except Exception:
                    self.item_buy_price_cache[name] = 0.0
                try:
                    self.item_sell_price_cache[name] = float(p['sell']['min'])
                except Exception:
                    self.item_sell_price_cache[name] = 0.0
            else:
                self.item_buy_price_cache[name] = 0.0
                self.item_sell_price_cache[name] = 0.0
        
        self._calculate_run_metrics()

        # 통계 데이터 반환
        daily_stats = {}
        for date, group in self.df.groupby('날짜'):
            daily_stats[date] = {
                "runs": group.to_dict(orient='records'),
                "avg_isk": group['실수익'].mean(),
                "avg_time": group['런 소요(분)'].mean(),
                "avg_iskph": group['ISK/h'].mean()
            }
        
        overall_stats = {}
        if not self.df.empty:
            overall_stats = {
                "avg_isk": self.df['실수익'].mean(),
                "avg_time": self.df['런 소요(분)'].mean(),
                "avg_iskph": self.df['ISK/h'].mean(),
                "tier_weather_stats": []
            }
            for (tier, weather), group in self.df.groupby(self.df['어비셜 종류'].str.split().str[0:2].apply(tuple)):
                if not group.empty:
                    overall_stats["tier_weather_stats"].append({
                        "tier": tier,
                        "weather": weather,
                        "runs_count": len(group),
                        "avg_isk": group['실수익'].mean(),
                        "avg_time": group['런 소요(분)'].mean(),
                        "avg_iskph": group['ISK/h'].mean()
                    })
        
        return {
            "daily_stats": daily_stats,
            "overall_stats": overall_stats
        }

    def analyze(self):
        self.console.print("\n📂 [1/5] CSV 파일 읽기...", style="bold")
        stats_data = self.get_analysis_data()
        if stats_data is None:
            self.console.print("[red]❌ 분석할 CSV 파일이 없습니다.[/red]")
            return
        
        self.console.print(f"  ▶️ {len(glob.glob(os.path.join(self.data_manager.data_dir, 'abyssal_results_*.csv')))}개 파일에서 런 데이터 수집 중...")
        
        all_item_names = set()
        for items in self.df['획득 아이템']:
            for name, qty in self.data_manager.parse_items(items):
                all_item_names.add(name)
        for abyssal_type in self.df['어비셜 종류']:
            filament = self.data_manager.abyssal_type_to_filament_name(abyssal_type)
            if filament:
                all_item_names.add(filament)
        all_item_names = list(all_item_names)
        self.console.print(f"  ▶️ {len(all_item_names)}종 아이템 발견!")

        self.console.print("\n🔎 [2/5] ESI API로 아이템 type_id 변환 중...")
        name_to_id = self.eve_api.fetch_type_ids(all_item_names)
        self.console.print(f"  ▶️ {len(name_to_id)}종 변환 성공! (미매칭: {len(all_item_names)-len(name_to_id)})")

        self.console.print("\n💰 [3/5] Fuzzwork로 대량 시세 조회 중...")
        ids = list(name_to_id.values())
        prices = {}
        for i in range(0, len(ids), 100):
            chunk = ids[i:i+100]
            prices.update(self.eve_api.fetch_fuzzwork_prices(chunk))
            time.sleep(0.2)

        self._print_statistics()

    def _calculate_run_metrics(self):
        def calc_run(row):
            items = self.data_manager.parse_items(row['획득 아이템'])
            total = sum(float(self.item_buy_price_cache.get(name, 0)) * int(qty) for name, qty in items)
            filament = self.data_manager.abyssal_type_to_filament_name(row['어비셜 종류'])
            filament_price = float(self.item_sell_price_cache.get(filament, 0)) * 3  # 프리깃 3배
            duration_min = float(row['런 소요(분)'])
            net_isk = total - filament_price
            isk_per_hour = net_isk / (duration_min / 60) if duration_min > 0 else 0
            return pd.Series({
                '드롭': total,
                '입장료': filament_price,
                '실수익': net_isk,
                'ISK/h': isk_per_hour
            })

        self.df[['드롭', '입장료', '실수익', 'ISK/h']] = self.df.apply(calc_run, axis=1)
        self.df['날짜'] = self.df['시작시각(KST)'].str[:10]

    def _print_statistics(self):
        self.console.print("\n==============================", style="bold")
        self.console.print("🗓️  날짜별 어비셜 런 통계 🗓️", style="bold")
        self.console.print("==============================", style="bold")
        for date, group in self.df.groupby('날짜'):
            table = Table(title=f"📅 {date}")
            table.add_column("시작시각")
            table.add_column("어비셜 종류")
            table.add_column("런 시간")
            table.add_column("💰 드롭", justify="right")
            table.add_column("🎫 입장료", justify="right")
            table.add_column("🟢 실수익", justify="right")
            table.add_column("🕒 ISK/h", justify="right")
            for _, row in group.iterrows():
                table.add_row(
                    row['시작시각(KST)'],
                    row['어비셜 종류'],
                    f"{row['런 소요(분)']:.2f}m",
                    f"{int(row['드롭']):,}",
                    f"-{int(row['입장료']):,}",
                    f"{int(row['실수익']):,}",
                    f"{int(row['ISK/h']):,}"
                )
            avg_isk = group['실수익'].mean()
            avg_time = group['런 소요(분)'].mean()
            avg_iskph = group['ISK/h'].mean()
            table.add_row("[평균]", "", f"{avg_time:.2f}m", "", "", f"{int(avg_isk):,}", f"{int(avg_iskph):,}")
            self.console.print(table)

        self.console.print("\n==============================", style="bold")
        self.console.print("📈 전체 어비셜 통계 📈", style="bold")
        self.console.print("==============================", style="bold")
        if not self.df.empty:
            avg_isk = self.df['실수익'].mean()
            avg_time = self.df['런 소요(분)'].mean()
            avg_iskph = self.df['ISK/h'].mean()
            self.console.print(f"[전체 평균]   🕒 {avg_time:.2f}m   🟢 {int(avg_isk):,}   🚀 {int(avg_iskph):,}")

            self.console.print("\n[티어/날씨별 통계]")
            table = Table()
            table.add_column("티어")
            table.add_column("날씨")
            table.add_column("런수", justify="right")
            table.add_column("🟢 평균실수익", justify="right")
            table.add_column("🕒 평균런시간", justify="right")
            table.add_column("🚀 평균ISK/h", justify="right")
            for (tier, weather), group in self.df.groupby(self.df['어비셜 종류'].str.split().str[0:2].apply(tuple)):
                if not group.empty:
                    table.add_row(
                        tier, weather, str(len(group)),
                        f"{int(group['실수익'].mean()):,}",
                        f"{group['런 소요(분)'].mean():.2f}",
                        f"{int(group['ISK/h'].mean()):,}"
                    )
            self.console.print(table)
        else:
            self.console.print("데이터 없음.")

if __name__ == "__main__":
    console = Console()
    eve_api = EVEApi()
    data_manager = AbyssalDataManager()
    analyzer = AbyssalPriceAnalyzer(eve_api, data_manager, console)
    analyzer.analyze()

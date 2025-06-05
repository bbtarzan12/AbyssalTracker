import pandas as pd
import time
from rich.console import Console

from src.logic.eve_api import EVEApi
from src.logic.abyssal_data_manager import AbyssalDataManager

class AbyssalDataAnalyzer:
    def __init__(self, eve_api: EVEApi, abyssal_data_manager: AbyssalDataManager):
        self.eve_api = eve_api
        self.data_manager = abyssal_data_manager
        self.console = Console()

    def analyze_data(self, status_placeholder=None):
        """
        모든 어비셜 런 데이터를 로드하고, 아이템 시세를 조회하여 분석합니다.
        status_placeholder는 Streamlit의 st.status 객체를 받아 진행 상황을 업데이트하는 데 사용됩니다.
        """
        start_total = time.time()
        
        if status_placeholder:
            status_placeholder.update(label="CSV 파일 로드 중... 📂", state="running", expanded=True)
        self.console.print("📂 [AbyssalDataAnalyzer] CSV 파일 로드 중...", style="bold")
        start_csv_load = time.time()
        df = self.data_manager.load_abyssal_results()
        
        end_csv_load = time.time()
        self.console.print(f"  ▶️ CSV 파일 로드 완료. 소요 시간: {end_csv_load - start_csv_load:.2f}초 ✅")
        if df.empty:
            if status_placeholder:
                status_placeholder.update(label="데이터 로딩 및 분석 완료 (데이터 없음) ⚠️", state="complete", expanded=False)
            self.console.print("[red]❌ 분석할 데이터가 없습니다.[/red]")
            return None, {}, {}, pd.DataFrame() # df, daily_stats, overall_stats, item_prices 반환

        if status_placeholder:
            status_placeholder.update(label="아이템 이름 수집 중... 🔍", state="running", expanded=True)
        self.console.print(f"  ▶️ 총 {len(df)}개의 런 데이터 로드 완료. ✅")
        self.console.print("  ▶️ 모든 아이템 이름 수집 중... 🔍")
        start_item_collection = time.time()
        all_item_names = set()
        for items in df['획득 아이템']:
            for name, qty in self.data_manager.parse_items(items):
                all_item_names.add(name)
        for abyssal_type in df['어비셜 종류']:
            filament = self.data_manager.abyssal_type_to_filament_name(abyssal_type)
            if filament:
                all_item_names.add(filament)
        all_item_names = list(all_item_names)
        end_item_collection = time.time()
        self.console.print(f"  ▶️ {len(all_item_names)}종의 아이템 발견! 소요 시간: {end_item_collection - start_item_collection:.2f}초 ✨")

        if status_placeholder:
            status_placeholder.update(label="아이템 type_id 변환 중 (ESI API)... 🔄", state="running", expanded=True)
        self.console.print("  ▶️ ESI API로 아이템 type_id 변환 중... 🔄")
        start_type_id_fetch = time.time()
        name_to_id = self.eve_api.fetch_type_ids(all_item_names)
        end_type_id_fetch = time.time()
        self.console.print(f"  ▶️ {len(name_to_id)}종 변환 성공! (미매칭: {len(all_item_names)-len(name_to_id)}) 소요 시간: {end_type_id_fetch - start_type_id_fetch:.2f}초 💡")

        if status_placeholder:
            status_placeholder.update(label="아이템 시세 조회 중 (Fuzzwork API)... 💰", state="running", expanded=True)
        self.console.print("  ▶️ Fuzzwork로 대량 시세 조회 중... 💰")
        start_price_fetch = time.time()
        ids = list(name_to_id.values())
        prices = {}
        for i, chunk_start in enumerate(range(0, len(ids), 100)):
            chunk = ids[chunk_start:chunk_start+100]
            prices.update(self.eve_api.fetch_fuzzwork_prices(chunk))
            time.sleep(0.05) # API 호출 간 지연 추가
        end_price_fetch = time.time()
        self.console.print(f"  ▶️ Fuzzwork 시세 조회 완료. 소요 시간: {end_price_fetch - start_price_fetch:.2f}초 💸")

        item_buy_price_cache = {}
        item_sell_price_cache = {}
        for name, type_id in name_to_id.items():
            p = prices.get(str(type_id))
            if p:
                try:
                    item_buy_price_cache[name] = float(p['buy']['max'])
                except (TypeError, ValueError, KeyError):
                    item_buy_price_cache[name] = 0.0
                    self.console.print(f"[WARNING] 아이템 '{name}'의 구매 가격 파싱 오류. 0으로 설정됨. 데이터: {p}", style="yellow")
                try:
                    item_sell_price_cache[name] = float(p['sell']['min'])
                except (TypeError, ValueError, KeyError):
                    item_sell_price_cache[name] = 0.0
                    self.console.print(f"[WARNING] 아이템 '{name}'의 판매 가격 파싱 오류. 0으로 설정됨. 데이터: {p}", style="yellow")
            else:
                item_buy_price_cache[name] = 0.0
                item_sell_price_cache[name] = 0.0
                self.console.print(f"[WARNING] 아이템 '{name}' ({type_id})에 대한 가격 데이터 없음. 0으로 설정됨.", style="yellow")
        
        # 런 지표 계산
        df['드롭'] = df['획득 아이템'].apply(
            lambda item_str: sum(
                float(item_buy_price_cache.get(name, 0)) * int(qty)
                for name, qty in self.data_manager.parse_items(item_str)
            )
        )

        df['입장료'] = df['어비셜 종류'].apply(
            lambda abyssal_type: float(item_sell_price_cache.get(
                self.data_manager.abyssal_type_to_filament_name(abyssal_type), 0
            )) * 3 # 프리깃 3배
        )

        df['실수익'] = df['드롭'] - df['입장료']
        df['ISK/h'] = df.apply(
            lambda row: row['실수익'] / (row['런 소요(분)'] / 60) if row['런 소요(분)'] > 0 else 0,
            axis=1
        )
        
        if status_placeholder:
            status_placeholder.update(label="데이터 분석 및 통계 생성 중... 📊", state="running", expanded=True)
        self.console.print("  ▶️ 런 지표 계산 및 통계 생성 중... 📊")
        start_analysis = time.time()
        df['날짜'] = df['시작시각(KST)'].str[:10]

        # 통계 데이터 생성
        daily_stats = {}
        for date, group in df.groupby('날짜'):
            daily_stats[date] = {
                "runs": group.to_dict(orient='records'),
                "avg_isk": group['실수익'].mean(),
                "avg_time": group['런 소요(분)'].mean(),
                "avg_iskph": group['ISK/h'].mean()
            }
        
        overall_stats = {}
        if not df.empty:
            overall_stats = {
                "avg_isk": df['실수익'].mean(),
                "avg_time": df['런 소요(분)'].mean(),
                "avg_iskph": df['ISK/h'].mean(),
                "tier_weather_stats": []
            }
            for (tier, weather), group in df.groupby(df['어비셜 종류'].str.split().str[0:2].apply(tuple)):
                if not group.empty:
                    overall_stats["tier_weather_stats"].append({
                        "tier": tier,
                        "weather": weather,
                        "runs_count": len(group),
                        "avg_isk": group['실수익'].mean(),
                        "avg_time": group['런 소요(분)'].mean(),
                        "avg_iskph": group['ISK/h'].mean()
                    })
        end_analysis = time.time()
        self.console.print(f"  ▶️ 데이터 분석 및 통계 생성 완료. 소요 시간: {end_analysis - start_analysis:.2f}초 ✅")

        end_total = time.time()
        self.console.print(f"✨ [AbyssalDataAnalyzer] 전체 데이터 로딩 및 분석 완료. 총 소요 시간: {end_total - start_total:.2f}초 ✨", style="bold green")
        
        if status_placeholder:
            status_placeholder.update(label="데이터 로딩 및 분석 완료! ✅", state="complete", expanded=False)
        return df, daily_stats, overall_stats, item_buy_price_cache # 모든 필요한 데이터 반환
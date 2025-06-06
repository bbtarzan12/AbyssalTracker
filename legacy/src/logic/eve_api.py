import os
import requests
import time
import json

class EVEApi:
    REGION_ID = 10000002  # The Forge
    STATION_ID = 60003760 # Jita 4-4
    CACHE_FILE = os.path.join('data', 'typeid_cache.json')

    def __init__(self):
        self.name_to_id_cache: dict[str, int] = self._load_cache()

    def _load_cache(self) -> dict[str, int]:
        try:
            with open(self.CACHE_FILE, 'r', encoding='utf-8') as f:
                return json.load(f)
        except (FileNotFoundError, json.JSONDecodeError):
            return {}

    def _save_cache(self):
        try:
            os.makedirs(os.path.dirname(self.CACHE_FILE), exist_ok=True)
            with open(self.CACHE_FILE, 'w', encoding='utf-8') as f:
                json.dump(self.name_to_id_cache, f, ensure_ascii=False, indent=2)
        except IOError as e:
            print(f"[ERROR] 캐시 파일 저장 중 오류 발생: {e}")

    def fetch_type_ids(self, names: list[str]) -> dict[str, int]:
        name_to_id: dict[str, int] = {}
        names_to_query = [n for n in names if n not in self.name_to_id_cache]

        if not names_to_query:
            print("[INFO] 모든 아이템 type_id가 캐시에 존재합니다. API 호출 건너뜀.")
            for n in names:
                if n in self.name_to_id_cache:
                    name_to_id[n] = self.name_to_id_cache[n]
            return name_to_id

        print(f"[INFO] ESI API로 {len(names_to_query)}개의 아이템 type_id 조회 시작...")
        for i in range(0, len(names_to_query), 20):
            print(f"[INFO] ESI API 호출 중... ({i+1}/{len(names_to_query)})")
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
                if not data:
                    print(f"[WARNING] ESI API 응답이 비어있습니다. 청크: {chunk}")
                    continue
                for itm in data.get("inventory_types", []):
                    self.name_to_id_cache[itm["name"]] = itm["id"]
                time.sleep(0.2)
            except requests.exceptions.Timeout:
                print(f"[ERROR] ESI API 호출 타임아웃 발생: {chunk}")
            except requests.exceptions.ConnectionError as e:
                print(f"[ERROR] ESI API 연결 오류 발생: {e}, 청크: {chunk}")
            except requests.exceptions.HTTPError as e:
                print(f"[ERROR] ESI API HTTP 오류 발생: {e.response.status_code} - {e.response.text}, 청크: {chunk}")
            except json.JSONDecodeError:
                print(f"[ERROR] ESI API 응답 JSON 디코딩 오류 발생. 응답: {resp.text}, 청크: {chunk}")
            except Exception as e: # Catch any other unexpected errors
                print(f"[ERROR] ESI API 호출 중 예상치 못한 오류 발생: {e}, 청크: {chunk}")
                continue
        
        self._save_cache()
        for n in names:
            if n in self.name_to_id_cache:
                name_to_id[n] = self.name_to_id_cache[n]
        print(f"[INFO] ESI API type_id 조회 완료. 총 {len(name_to_id)}개 변환 성공.")
        return name_to_id

    def fetch_fuzzwork_prices(self, ids: list[int]) -> dict:
        if not ids:
            print("[INFO] Fuzzwork API에 조회할 type_id가 없습니다.")
            return {}

        url = (f"https://market.fuzzwork.co.uk/aggregates/?region={self.REGION_ID}"
               f"&types={','.join(map(str, ids))}")
        try:
            resp = requests.get(url, timeout=15)
            resp.raise_for_status()
            data = resp.json()
            if not data:
                print(f"[WARNING] Fuzzwork API 응답이 비어있습니다. URL: {url}")
                return {}
            print(f"[INFO] Fuzzwork API 시세 조회 성공. {len(data)}개 아이템.")
            return data
        except requests.exceptions.Timeout:
            print(f"[ERROR] Fuzzwork API 호출 타임아웃 발생: {url}")
            return {}
        except requests.exceptions.ConnectionError as e:
            print(f"[ERROR] Fuzzwork API 연결 오류 발생: {e}, URL: {url}")
            return {}
        except requests.exceptions.HTTPError as e:
                print(f"[ERROR] Fuzzwork API HTTP 오류 발생: {e.response.status_code} - {e.response.text}, URL: {url}")
        except json.JSONDecodeError:
            print(f"[ERROR] Fuzzwork API 응답 JSON 디코딩 오류 발생. 응답: {resp.text}, URL: {url}")
        except Exception as e: # Catch any other unexpected errors
            print(f"[ERROR] Fuzzwork API 호출 중 오류 발생: {e}")
            return {}
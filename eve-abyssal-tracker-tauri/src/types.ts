// 공통 타입 정의

export interface RunData {
  '시작시각(KST)': string;
  '종료시각(KST)': string;
  '런 소요(분)': number;
  '어비셜 종류': string;
  '함급'?: number; // 옵셔널로 처리
  '실수익': number;
  'ISK/h': number;
  '획득 아이템': string;
  '날짜': string;
}

export interface DailyStats {
  [date: string]: {
    runs: RunData[];
    avg_isk: number;
    avg_time: number;
    avg_iskph: number;
  };
}

export interface OverallStats {
  avg_isk: number;
  avg_time: number;
  avg_iskph: number;
  tier_weather_stats: {
    tier: string;
    weather: string;
    runs_count: number;
    avg_isk: number;
    avg_time: number;
    avg_iskph: number;
  }[];
}

export interface AbyssalData {
  df: RunData[];
  daily_stats: DailyStats;
  overall_stats: OverallStats;
  item_buy_price_cache: { [key: string]: number };
}

export interface LocationInfo {
  current_system: string | null;
  previous_system: string | null;
  last_updated: string | null;
} 
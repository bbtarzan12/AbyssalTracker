import tkinter as tk
from tkinter import ttk
import pandas as pd

class AbyssalStatsDisplayPopup(tk.Toplevel):
    _parent_root = None

    @classmethod
    def set_parent_root(cls, root):
        cls._parent_root = root

    def __init__(self, stats_data=None):
        if not self._parent_root:
            raise RuntimeError("Parent root not set. Call set_parent_root() first.")
        
        super().__init__(self._parent_root)
        self.title("어비셜 런 통계")
        self.geometry("800x600")
        self.stats_data = stats_data
        self.content_frame = None # 통계 내용을 담을 프레임
        
        if self.stats_data:
            self._create_widgets()
        else:
            self._create_loading_widgets()
        self.deiconify() # 팝업을 화면에 표시

    def _create_loading_widgets(self):
        # 기존 위젯 제거 (업데이트 시 호출될 경우를 대비)
        for widget in self.winfo_children():
            widget.destroy()

        self.loading_label = tk.Label(self, text="데이터 로딩 중...", font=("Helvetica", 16, "bold"))
        self.loading_label.pack(expand=True)

    def update_content(self, stats_data):
        # 기존 로딩 위젯 제거
        if hasattr(self, 'loading_label') and self.loading_label.winfo_exists():
            self.loading_label.destroy()
        
        # 기존 통계 내용 위젯 제거 (재로드 시 호출될 경우를 대비)
        if self.content_frame and self.content_frame.winfo_exists():
            self.content_frame.destroy()

        self.stats_data = stats_data
        self._create_widgets()

    def _create_widgets(self):
        # 통계 내용을 담을 새로운 프레임 생성
        self.content_frame = ttk.Frame(self)
        self.content_frame.pack(expand=True, fill="both")

        # 탭 컨트롤 생성
        self.notebook = ttk.Notebook(self.content_frame)
        self.notebook.pack(expand=True, fill="both", padx=10, pady=10)

        # 일별 통계 탭
        daily_frame = ttk.Frame(self.notebook)
        self.notebook.add(daily_frame, text="일별 통계")
        self._create_daily_stats_tab(daily_frame)

        # 전체 통계 탭
        overall_frame = ttk.Frame(self.notebook)
        self.notebook.add(overall_frame, text="전체 통계")
        self._create_overall_stats_tab(overall_frame)

    def _create_daily_stats_tab(self, parent_frame):
        if not self.stats_data or not self.stats_data.get("daily_stats"):
            tk.Label(parent_frame, text="일별 통계 데이터가 없습니다.").pack(pady=20)
            return

        canvas = tk.Canvas(parent_frame)
        scrollbar = ttk.Scrollbar(parent_frame, orient="vertical", command=canvas.yview)
        scrollable_frame = ttk.Frame(canvas)

        scrollable_frame.bind(
            "<Configure>",
            lambda e: canvas.configure(
                scrollregion=canvas.bbox("all")
            )
        )

        canvas.create_window((0, 0), window=scrollable_frame, anchor="nw")
        canvas.configure(yscrollcommand=scrollbar.set)

        canvas.pack(side="left", fill="both", expand=True)
        scrollbar.pack(side="right", fill="y")

        for date, data in self.stats_data["daily_stats"].items():
            tk.Label(scrollable_frame, text=f"📅 {date}", font=("Helvetica", 12, "bold")).pack(anchor="w", pady=(10, 0))
            
            # 일별 런 데이터 테이블
            tree = ttk.Treeview(scrollable_frame, columns=("시작시각", "어비셜 종류", "런 시간", "드롭", "입장료", "실수익", "ISK/h"), show="headings")
            tree.heading("시작시각", text="시작시각")
            tree.heading("어비셜 종류", text="어비셜 종류")
            tree.heading("런 시간", text="런 시간")
            tree.heading("드롭", text="💰 드롭")
            tree.heading("입장료", text="🎫 입장료")
            tree.heading("실수익", text="🟢 실수익")
            tree.heading("ISK/h", text="🕒 ISK/h")

            tree.column("시작시각", width=150, anchor="center")
            tree.column("어비셜 종류", width=100, anchor="center")
            tree.column("런 시간", width=70, anchor="center")
            tree.column("드롭", width=100, anchor="e")
            tree.column("입장료", width=100, anchor="e")
            tree.column("실수익", width=100, anchor="e")
            tree.column("ISK/h", width=100, anchor="e")

            for run in data["runs"]:
                tree.insert("", "end", values=(
                    run['시작시각(KST)'],
                    run['어비셜 종류'],
                    f"{run['런 소요(분)']:.2f}m",
                    f"{int(run['드롭']):,}",
                    f"-{int(run['입장료']):,}",
                    f"{int(run['실수익']):,}",
                    f"{int(run['ISK/h']):,}"
                ))
            
            tree.pack(fill="x", expand=True, pady=5)

            # 일별 평균 요약
            avg_frame = tk.Frame(scrollable_frame)
            avg_frame.pack(fill="x", pady=5)
            tk.Label(avg_frame, text=f"[평균] 런 시간: {data['avg_time']:.2f}m, 실수익: {int(data['avg_isk']):,} ISK, ISK/h: {int(data['avg_iskph']):,}").pack(anchor="w")
            tk.Frame(scrollable_frame, height=1, bg="gray").pack(fill="x", pady=5) # 구분선

    def _create_overall_stats_tab(self, parent_frame):
        if not self.stats_data or not self.stats_data.get("overall_stats"):
            tk.Label(parent_frame, text="전체 통계 데이터가 없습니다.").pack(pady=20)
            return

        overall = self.stats_data["overall_stats"]

        # 전체 평균 요약
        tk.Label(parent_frame, text="📈 전체 어비셜 통계 📈", font=("Helvetica", 14, "bold")).pack(pady=(10, 5))
        tk.Label(parent_frame, text=f"[전체 평균] 런 시간: {overall['avg_time']:.2f}m, 실수익: {int(overall['avg_isk']):,} ISK, ISK/h: {int(overall['avg_iskph']):,}").pack(pady=5)

        tk.Label(parent_frame, text="\n[티어/날씨별 통계]", font=("Helvetica", 12, "bold")).pack(pady=(10, 5), anchor="w")
        
        # 티어/날씨별 통계 테이블
        tree = ttk.Treeview(parent_frame, columns=("티어", "날씨", "런수", "평균실수익", "평균런시간", "평균ISK/h"), show="headings")
        tree.heading("티어", text="티어")
        tree.heading("날씨", text="날씨")
        tree.heading("런수", text="런수")
        tree.heading("평균실수익", text="🟢 평균실수익")
        tree.heading("평균런시간", text="🕒 평균런시간")
        tree.heading("평균ISK/h", text="🚀 평균ISK/h")

        tree.column("티어", width=70, anchor="center")
        tree.column("날씨", width=70, anchor="center")
        tree.column("런수", width=70, anchor="e")
        tree.column("평균실수익", width=120, anchor="e")
        tree.column("평균런시간", width=120, anchor="e")
        tree.column("평균ISK/h", width=120, anchor="e")

        for stats in overall["tier_weather_stats"]:
            tree.insert("", "end", values=(
                stats['tier'],
                stats['weather'],
                stats['runs_count'],
                f"{int(stats['avg_isk']):,}",
                f"{stats['avg_time']:.2f}",
                f"{int(stats['avg_iskph']):,}"
            ))
        tree.pack(fill="both", expand=True, padx=10, pady=5)

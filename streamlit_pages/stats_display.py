import streamlit as st
import pandas as pd
import plotly.express as px
import sys
import os
import time
from rich.console import Console

# 프로젝트 루트 디렉토리를 sys.path에 추가하여 src/logic 모듈을 임포트할 수 있도록 합니다.
# 이 스크립트가 abyssal_v2/streamlit_pages/stats_display.py 에 위치한다고 가정합니다.
project_root = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
if project_root not in sys.path:
    sys.path.insert(0, project_root)

from src.logic.abyssal_data_manager import AbyssalDataManager
from src.logic.eve_api import EVEApi # EVEApi 임포트 추가
from src.logic.abyssal_data_analyzer import AbyssalDataAnalyzer # AbyssalDataAnalyzer 임포트

def app():
    st.title("📊 어비셜 런 통계")

    # AbyssalDataAnalyzer 인스턴스를 Streamlit 세션 상태에 저장
    # 이렇게 하면 앱이 리런될 때마다 새로 생성되지 않고 기존 인스턴스를 재사용합니다.
    if 'abyssal_data_analyzer' not in st.session_state:
        st.session_state.abyssal_data_analyzer = AbyssalDataAnalyzer(eve_api=EVEApi(), abyssal_data_manager=AbyssalDataManager())
    
    abyssal_data_analyzer = st.session_state.abyssal_data_analyzer

    # 데이터 로딩 및 분석
    def load_and_analyze_data():
        with st.status("데이터 로딩 및 분석 중... 🚀", expanded=True) as status:
            # AbyssalDataAnalyzer의 데이터 분석 메서드 호출
            # status 객체를 analyze_data 메서드의 status_placeholder 인자로 전달
            df, daily_stats, overall_stats, item_buy_price_cache = abyssal_data_analyzer.analyze_data(status_placeholder=status)
            
            if df is None or df.empty:
                status.update(label="데이터 로딩 및 분석 완료 (데이터 없음) ⚠️", state="complete", expanded=False)
            else:
                status.update(label="데이터 로딩 및 분석 완료! ✅", state="complete", expanded=False)
        return df, daily_stats, overall_stats, item_buy_price_cache # 필요한 모든 데이터 반환

    # 데이터 로딩 및 분석 실행
    # 데이터가 세션 상태에 없거나 새로고침이 필요한 경우에만 로드
    if 'df' not in st.session_state or st.button("데이터 새로고침 🔄"):
        st.session_state.df, st.session_state.daily_stats, st.session_state.overall_stats, st.session_state.item_buy_price_cache = load_and_analyze_data()
        st.session_state.data_loaded = True
    
    df = st.session_state.df
    daily_stats = st.session_state.daily_stats
    overall_stats = st.session_state.overall_stats
    item_buy_price_cache = st.session_state.item_buy_price_cache

    if df is None or df.empty:
        st.warning("⚠️ 분석할 어비셜 런 데이터가 없습니다. 런을 기록한 후 다시 시도해주세요.")
        st.session_state.data_loaded = False # 데이터 없으면 로드 안된 상태로 유지
    else:
        st.session_state.data_loaded = True
        st.markdown("---")
        
    # 데이터 로딩 완료 후에만 새로고침 버튼 표시 (이미 위에서 처리됨)
    # if st.session_state.data_loaded:
    #     if st.button("데이터 새로고침 🔄"):
    #         st.rerun() # 페이지 다시 실행

        tab1, tab2 = st.tabs(["🗓️ 일별 통계", "📈 전체 통계"])

        with tab1:
            st.header("🗓️ 일별 어비셜 런 통계")
            
            if not daily_stats:
                st.info("일별 통계 데이터가 없습니다.")
            else:
                # 날짜 선택 드롭다운
                selected_date = st.selectbox("날짜 선택", sorted(daily_stats.keys(), reverse=True))
                
                if selected_date:
                    date_data = daily_stats[selected_date]
                    st.subheader(f"📅 {selected_date} 런 상세")
                    
                    # 총 얻은 수익 추가
                    total_daily_isk = sum(run['실수익'] for run in date_data['runs'])
                    st.metric(label=f"{selected_date} 총 얻은 수익", value=f"{int(total_daily_isk):,} ISK 💰")

                    # 일별 런 데이터 상세 표시 (획득 아이템 개선)
                    for i, run in enumerate(date_data["runs"]):
                        with st.expander(f"런 {i+1}: {run['시작시각(KST)']} - {run['어비셜 종류']} ({int(run['실수익']):,} ISK)"):
                            col_info1, col_info2 = st.columns(2)
                            with col_info1:
                                st.write(f"**⏰ 시작 시각:** {run['시작시각(KST)']}")
                                st.write(f"**🏁 종료 시각:** {run['종료시각(KST)']}")
                                st.write(f"**⏱️ 런 소요:** {run['런 소요(분)']:.2f} 분")
                            with col_info2:
                                st.write(f"**🌌 어비셜 종류:** {run['어비셜 종류']}")
                                st.write(f"**💰 실수익:** {int(run['실수익']):,} ISK")
                                st.write(f"**🚀 ISK/h:** {int(run['ISK/h']):,} ISK/h")
                            
                            st.markdown("---")
                            st.markdown("### 📦 획득 아이템")
                            if run['획득 아이템']:
                                parsed_items = abyssal_data_analyzer.data_manager.parse_items(run['획득 아이템'])
                                # 아이템 이름별로 수량과 총 가격을 합산
                                aggregated_items = {}
                                for item_name, item_qty in parsed_items:
                                    price = item_buy_price_cache.get(item_name, 0)
                                    total_price = price * item_qty
                                    if item_name in aggregated_items:
                                        aggregated_items[item_name]["개수"] += item_qty
                                        aggregated_items[item_name]["총 가격_num"] += total_price
                                    else:
                                        aggregated_items[item_name] = {
                                            "아이템 이름": item_name,
                                            "개수": item_qty,
                                            "개당 가격": price, # 나중에 포맷팅
                                            "총 가격_num": total_price # 정렬을 위해 숫자 값 유지
                                        }
                                
                                item_data = []
                                for name, data in aggregated_items.items():
                                    item_data.append({
                                        "아이템 이름": data["아이템 이름"],
                                        "개수": data["개수"],
                                        "개당 가격": f"{int(data['개당 가격']):,}",
                                        "총 가격": f"{int(data['총 가격_num']):,}"
                                    })
                                
                                if item_data:
                                    item_df = pd.DataFrame(item_data)
                                    item_df['총 가격_num'] = item_df['총 가격'].str.replace(',', '').astype(int)
                                    item_df = item_df.sort_values(by='총 가격_num', ascending=False).drop(columns='총 가격_num')
                                    st.dataframe(item_df, use_container_width=True, hide_index=True)
                                else:
                                    st.info("획득 아이템이 없습니다. 😥")
                            else:
                                st.info("획득 아이템이 없습니다. 😥")
                    
                    st.markdown("---")
                    col1, col2, col3 = st.columns(3)
                    with col1:
                        st.metric(label="평균 실수익", value=f"{int(date_data['avg_isk']):,} ISK 💰")
                    with col2:
                        st.metric(label="평균 런 소요 시간", value=f"{date_data['avg_time']:.2f} 분 ⏱️")
                    with col3:
                        st.metric(label="평균 ISK/h", value=f"{int(date_data['avg_iskph']):,} ISK/h 🚀")

                    st.markdown("---")
                    st.subheader("일별 런 실수익 추이")
                    # 선택된 날짜의 데이터만 필터링하여 그래프 생성
                    filtered_df_daily = df[df['날짜'] == selected_date]
                    fig_daily_isk = px.line(
                        filtered_df_daily,
                        x="시작시각(KST)",
                        y="실수익",
                        title=f"{selected_date} 일별 런 실수익",
                        labels={"시작시각(KST)": "시간", "실수익": "실수익 (ISK)"},
                        markers=True
                    )
                    st.plotly_chart(fig_daily_isk, use_container_width=True)

        with tab2:
            st.header("📈 전체 어비셜 런 통계")
            
            if not overall_stats:
                st.info("전체 통계 데이터가 없습니다.")
            else:
                col1, col2, col3 = st.columns(3)
                with col1:
                    st.metric(label="전체 평균 실수익", value=f"{int(overall_stats['avg_isk']):,} ISK 💰")
                with col2:
                    st.metric(label="전체 평균 런 소요 시간", value=f"{overall_stats['avg_time']:.2f} 분 ⏱️")
                with col3:
                    st.metric(label="전체 평균 ISK/h", value=f"{int(overall_stats['avg_iskph']):,} ISK/h 🚀")

                st.markdown("---")
                st.subheader("티어/날씨별 통계")
                tier_weather_df = pd.DataFrame(overall_stats["tier_weather_stats"])
                if not tier_weather_df.empty:
                    st.dataframe(tier_weather_df, use_container_width=True)

                    st.markdown("---")
                    st.subheader("티어별 평균 실수익")
                    fig_tier_isk = px.bar(
                        tier_weather_df,
                        x="tier",
                        y="avg_isk",
                        color="weather",
                        title="티어별 평균 실수익",
                        labels={"tier": "티어", "avg_isk": "평수익 (ISK)"}
                    )
                    st.plotly_chart(fig_tier_isk, use_container_width=True)

                    st.markdown("---")
                    st.subheader("날씨별 평균 ISK/h")
                    fig_weather_iskph = px.bar(
                        tier_weather_df,
                        x="weather",
                        y="avg_iskph",
                        color="tier",
                        title="날씨별 평균 ISK/h",
                        labels={"weather": "날씨", "avg_iskph": "평균 ISK/h"}
                    )
                    st.plotly_chart(fig_weather_iskph, use_container_width=True)
                else:
                    st.info("티어/날씨별 통계 데이터가 없습니다.")

                st.markdown("---")
                st.subheader("전체 런 실수익 분포")
                fig_overall_isk_hist = px.histogram(
                    df,
                    x="실수익",
                    nbins=50,
                    title="전체 런 실수익 분포",
                    labels={"실수익": "실수익 (ISK)"}
                )
                st.plotly_chart(fig_overall_isk_hist, use_container_width=True)

                st.markdown("---")
                st.subheader("전체 런 소요 시간 분포")
                fig_overall_time_hist = px.histogram(
                    df,
                    x="런 소요(분)",
                    nbins=50,
                    title="전체 런 소요 시간 분포",
                    labels={"런 소요(분)": "런 소요 시간 (분)"}
                )
                st.plotly_chart(fig_overall_time_hist, use_container_width=True)
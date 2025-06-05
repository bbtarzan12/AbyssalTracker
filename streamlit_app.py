import streamlit as st
import sys
import os

# 프로젝트 루트 디렉토리를 sys.path에 추가하여 src/logic 모듈을 임포트할 수 있도록 합니다.
# 이 스크립트가 abyssal_v2/streamlit_app.py 에 위치한다고 가정합니다.
project_root = os.path.abspath(os.path.join(os.path.dirname(__file__), '.'))
if project_root not in sys.path:
    sys.path.insert(0, project_root)

import streamlit as st
import sys
import os

# 프로젝트 루트 디렉토리를 sys.path에 추가하여 src/logic 모듈을 임포트할 수 있도록 합니다.
# 이 스크립트가 abyssal_v2/streamlit_app.py 에 위치한다고 가정합니다.
project_root = os.path.abspath(os.path.join(os.path.dirname(__file__), '.'))
if project_root not in sys.path:
    sys.path.insert(0, project_root)

# Streamlit 페이지 임포트
from streamlit_pages import stats_display

st.set_page_config(
    page_title="EVE Abyssal Tracker",
    page_icon="🚀",
    layout="wide",
    initial_sidebar_state="expanded"
)

st.sidebar.title("메뉴")
st.sidebar.markdown("---")

# 페이지 선택 (임시)
page = st.sidebar.radio(
    "페이지 선택",
    ["📊 통계"]
)

if page == "📊 통계":
    stats_display.app()

st.sidebar.markdown("---")
st.sidebar.info("EVE Abyssal Tracker v2.0")
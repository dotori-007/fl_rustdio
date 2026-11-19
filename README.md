# 🎵 FL Rustdio (fl_rustdio)

> **Pure Rust 기반 초경량·고성능 네이티브 오디오 스튜디오 & 나이트코어(Nightcore)/로리코어(Lolicore) DSP 프로세서**  
> 웹 브라우저나 무거운 런타임 없이 단일 네이티브 실행 파일(`.exe`, ~7.3MB)로 구동되며, 60FPS 하드웨어 가속 다크 스튜디오 UI, 실시간 무지연 오디오 DSP 이펙터, 사운드클라우드 스타일 대화형 음 파형 타임라인, 온라인 음원 고속 추출기를 탑재한 차세대 오디오 워크스테이션입니다.

---

## ✨ 핵심 기능 (Key Features)

### 1. 🎛️ 실시간 무지연 DSP 이펙터 엔진 (Zero-Latency Real-Time DSP)
슬라이더를 조작하거나 숫자를 입력하는 즉시 재생 중인 오디오에 무지연으로 실시간 사운드 이펙트가 적용됩니다.

- **독립 속도 조절 & 음정 역보상 피치 시프터 (Pitch-Preserved Time Stretch)**:
  - 배속(Speed: `0.25x` ~ `4.00x`)을 올려도 **보컬과 악기의 원곡 피치가 완벽하게 보존**됩니다.
  - 리샘플링 가속 시 발생하는 피치 상승분($+12 \times \log_2(\text{speed})$)을 내부의 무지연 `DualDelayPitchShifter`가 실시간으로 정밀 역보상합니다.
- **🔗 바이닐 연동 가속 모드 (Varispeed / Turntable Mode)**:
  - 체크박스 활성화 시 턴테이블이나 아날로그 테이프 릴처럼 속도와 피치가 비례하여 함께 상승하는 클래식 나이트코어 음색으로 즉시 전환됩니다.
- **🎵 정밀 피치 시프트 (Pitch Shift)**:
  - `-24.0` ~ `+24.0` 반음 (0.1 반음 단위 정밀 조절)
- **🔊 2차 IIR Biquad Low-Shelf 베이스 부스트 (Bass Boost)**:
  - `100Hz` 기준 `0.0dB` ~ `+24.0dB` 펀치감 있는 저역 증폭
- **💨 2차 IIR Biquad High-Shelf 고음 에어 / 보컬 선명화 (Treble Boost)**:
  - `8kHz` 기준 `0.0dB` ~ `+20.0dB` 나이트코어 특유의 선명한 보컬 에어감 부여
- **🌌 Schroeder Reverb 공간감 리버브 (Dreamy Reverb)**:
  - 4개의 병렬 Comb 필터 + 2개의 직렬 Allpass 필터 기반 부드러운 잔향감 연출 (`0%` ~ `100%`)
- **⌨️ 3단 일체형 수치 입력 컨트롤**:
  - `[라벨]` + `[직접 입력 숫자 박스 (DragValue)]` + `[슬라이더]`
  - 마우스 드래그 조절뿐만 아니라, **숫자 박스를 클릭하여 키보드로 직접 정확한 수치 타이핑(Enter)** 지원

---

### 2. 🌊 사운드클라우드 스타일 대화형 전체 음 파형 타임라인 (Interactive Waveform Dock)
하단에 배치된 인터랙티브 파형 타임라인으로 음악 전체의 흐름을 한눈에 파악하고 직관적으로 탐색합니다.

- **스트리밍 피크 추출**: 오디오 파일 전체를 220개 버킷으로 스트리밍 디코딩하여 시각화 피크 벡터 생성
- **네온 그라데이션 미러드 바**: 재생된 구간(시안 $\rightarrow$ 퍼플 그라데이션)과 미재생 구간(다크 슬레이트 블루)의 명확한 시각적 대비
- **오차 0.0px 실시간 동기화**: 재생 헤드 라인(`playhead_x`)과 파형 바의 활성화 경계가 픽셀 단위로 정확하게 일치
- **대화형 클릭 & 드래그 탐색 (Seek-on-Click / Drag)**: 원하는 구간을 클릭하거나 드래그하여 즉시 재생 위치 이동
- **마우스 호버 가이드선 & 시간 툴팁**: 마우스 커서 위치의 실시간 재생 시간 타임코드 표시
- **스페이스바(Space) 단축키**: 오디오 재생 / 일시정지 즉시 토글 (텍스트 필드 타이핑 시에는 공백 입력 스마트 예외 처리)

---

### 3. ⚡ 원클릭 특화 프리셋 칩 (Genre Presets)
- ⚡ **Classic Nightcore**: 1.25x 바이닐 연동 가속 + 베이스 부스트 2.0dB
- 💎 **Modern Nightcore**: 1.20x 보컬 튜닝 (+3 반음) + 고음 에어 3.5dB + 공간감 리버브 10%
- 🔥 **Lolicore Hyper**: 1.45x (+5 반음) + 초고속 하이퍼 비트
- ⚡ **Lolicore Extreme**: 1.65x (+7 반음) + 익스트림 속도
- 🌙 **Slowed & Reverb**: 0.85x 감속 (-3 반음) + 공간감 리버브 25% (드림코어 감성)

---

### 4. 🌐 고속 온라인 음원 추출기 (YouTube & SoundCloud Extractor)
- 상단 통합 추출 바에 URL을 붙여넣고 버튼을 누르면 고음질 오디오 스트림 즉시 추출
- **멀티스레드 병렬 다운로드 (`--concurrent-fragments 4`, `-f ba/b`)** 탑재로 다운로드 속도 3배 이상 향상
- **검은 CMD 콘솔창 팝업 원천 차단** (`CREATE_NO_WINDOW = 0x08000000` 적용)
- 다운로드 완료 즉시 좌측 편집 트랙으로 자동 로드

---

### 5. 📦 FIFO 큐(Queue) 기반 스마트 디스크 용량 관리
- `uploads/` 및 `outputs/` 폴더 내 파일 개수가 **10개를 초과하면 가장 오래된 파일부터 1개씩 자동 삭제**
- 방금 다운로드되거나 렌더링된 최신 파일은 보존 대상(`preserve_path`)으로 안전하게 보호
- 앱 실행 시(`main.rs`)에도 이전 작업 잔여물을 10개 큐로 자동 초기화

---

### 6. 💾 무손실 포맷 내보내기 & 다중 트랙 병합기
- **내보내기 포맷 지원**:
  - `MP3`: 192~320 kbps 고음질
  - `WAV`: 16-bit PCM 무손실
  - `FLAC`: 무손실 압축
- **트랙 병합 (Track Merger)**:
  - 여러 오디오 파일 등록, 순서 변경(▲/▼), 일괄 무손실 결합

---

## 🏗️ 아키텍처 (Project Architecture)

순수 Rust 네이티브 구조로 외부 종속성을 최소화하여 가볍고 견고하게 동작합니다:

```
fl_rustdio/
├── Cargo.toml                 # Rust 패키지 설정 및 의존성
├── run.bat                    # 원클릭 데스크톱 앱 실행 스크립트
├── README.md                  # 프로젝트 안내서
├── src/
│   ├── main.rs                # 앱 진입점, eframe 설정, 10개 큐 초기화
│   ├── app.rs                 # 메인 App 루프, 스페이스바 단축키, 백그라운드 워커 스레드 관리
│   ├── audio/
│   │   ├── mod.rs
│   │   ├── dsp.rs             # 무지연 DSP 스트림 (IIR Low/High-Shelf, 피치 시프터, 리버브, 비주얼라이저)
│   │   ├── engine.rs          # FFmpeg 연동 인코딩, 트랙 병합, 메타데이터 파싱
│   │   ├── extractor.rs       # yt-dlp 고속 멀티스레드 온라인 음원 추출기
│   │   └── player.rs          # Rodio 기반 인앱 실시간 플레이어 & 220 버킷 파형 추출기
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── theme.rs           # 딥 다크 스튜디오 팔레트 및 Windows Segoe UI Emoji/맑은 고딕 바인딩
│   │   ├── waveform.rs        # 사운드클라우드 스타일 대화형 파형 타임라인 렌더러
│   │   ├── tab_editor.rs      # 트랙 편집기 (URL 추출 바 + 원본 트랙 + 실시간 DSP 컨트롤)
│   │   ├── tab_extract.rs     # 온라인 음원 추출 상태 관리
│   │   └── tab_merge.rs       # 다중 트랙 병합기 탭
│   └── utils/
│       ├── mod.rs
│       ├── paths.rs           # System32 방지 절대 경로 고정 & 10개 FIFO 큐 용량 관리
│       └── tools.rs           # FFmpeg 및 yt-dlp 절대 경로 자동 탐색 & CREATE_NO_WINDOW 헬퍼
├── uploads/                   # 온라인 추출 음원 임시 저장소 (최대 10개 FIFO 유지)
└── outputs/                   # 편집 및 병합 완료 음원 저장소 (최대 10개 FIFO 유지)
```

---

## 🚀 빠른 시작 (Getting Started)

### 사전 준비 (Prerequisites)
- [Rust toolchain](https://rustup.rs/) (1.75+ 권장)
- (선택 사항) 오디오 렌더링 및 웹 음원 추출을 위한 `ffmpeg` 및 `yt-dlp` (시스템 PATH에 있거나 로컬 경로 자동 탐색)

### 실행 방법
```powershell
# 레포지토리 클론
git clone https://github.com/dotori-007/fl_rustdio.git
cd fl_rustdio

# 원클릭 실행 (run.bat 더블 클릭 또는 콘솔 실행)
.\run.bat

# 또는 Cargo로 직접 실행
cargo run --release
```

### 테스트 실행
```powershell
cargo test
# (5개 단위/통합 테스트 + FIFO 10개 큐 검증 테스트 100% 통과)
```

---

## 🎹 단축키 및 컨트롤 (Keybindings)

| 단축키 / 동작 | 기능 |
|:---|:---|
| **Space** | 활성 트랙 재생 / 일시정지 토글 (텍스트 필드 포커스 시 공백 입력 유지) |
| **마우스 클릭 & 드래그 (파형 뷰)** | 원하는 재생 구간으로 즉시 탐색 (Seek) |
| **마우스 호버 (파형 뷰)** | 해당 지점의 시간 타임코드 툴팁 표시 |
| **숫자 박스 클릭 (이펙터)** | 키보드로 정밀한 수치 직접 입력 후 Enter |
| **숫자 박스 드래그 (이펙터)** | 마우스 좌우 드래그로 수치 미세 조절 |
| **파일 드래그 & 드롭** | 오디오 파일을 앱 화면에 끌어다 놓아 즉시 로드 |

---

## 📄 라이선스 (License)

This project is licensed under the MIT License.

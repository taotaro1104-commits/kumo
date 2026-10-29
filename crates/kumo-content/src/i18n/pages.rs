//! 内部ページ（新しいタブ・設定・ブックマーク・パスワード・メモリ）の文言。

messages! {
    Pg {
        // ---- 新しいタブ ----
        NewTab => [
            "新しいタブ",
            "New tab",
            "新标签页",
            "새 탭",
            "Nueva pestaña"
        ],
        Lead => [
            "上の欄に URL か調べたい言葉を入力してください",
            "Type a URL or something to search for in the box above",
            "请在上方输入网址或要搜索的内容",
            "위 입력란에 URL이나 검색어를 입력하세요",
            "Escribe una URL o una búsqueda en el cuadro de arriba"
        ],
        Frequent => [
            "よく見るページ",
            "Frequently visited",
            "常去的网站",
            "자주 방문한 페이지",
            "Más visitadas"
        ],
        NoHistory => [
            "まだ履歴がありません。開いたページがここに並びます。",
            "No history yet. Pages you open will appear here.",
            "还没有历史记录。打开过的页面会显示在这里。",
            "아직 기록이 없습니다. 열어 본 페이지가 여기에 표시됩니다.",
            "Aún no hay historial. Las páginas que abras aparecerán aquí."
        ],
        KeyHome => ["ホームへ", "Go home", "回到主页", "홈으로", "Ir al inicio"],
        KeyBookmark => [
            "ブックマーク",
            "Bookmark",
            "加书签",
            "북마크",
            "Marcador"
        ],
        KeyBookmarks => [
            "ブックマーク一覧",
            "Bookmark list",
            "书签列表",
            "북마크 목록",
            "Lista de marcadores"
        ],
        KeyUrlBar => [
            "URL 欄へ",
            "Address bar",
            "地址栏",
            "주소 표시줄",
            "Barra de direcciones"
        ],
        KeyNewTab => [
            "新しいタブ",
            "New tab",
            "新标签页",
            "새 탭",
            "Nueva pestaña"
        ],
        KeyCloseTab => [
            "タブを閉じる",
            "Close tab",
            "关闭标签页",
            "탭 닫기",
            "Cerrar pestaña"
        ],
        KeyBack => ["戻る", "Back", "后退", "뒤로", "Atrás"],
        KeyClick => [
            "リンクを背景のタブで開く",
            "Open a link in a background tab",
            "在后台标签页中打开链接",
            "링크를 백그라운드 탭에서 열기",
            "Abrir un enlace en segundo plano"
        ],
        KeyClickLabel => [
            "Ctrl+クリック",
            "Ctrl+click",
            "Ctrl+点击",
            "Ctrl+클릭",
            "Ctrl+clic"
        ],
        Settings => ["設定", "Settings", "设置", "설정", "Ajustes"],
        Bookmarks => [
            "ブックマーク",
            "Bookmarks",
            "书签",
            "북마크",
            "Marcadores"
        ],
        Passwords => [
            "パスワード",
            "Passwords",
            "密码",
            "비밀번호",
            "Contraseñas"
        ],
        Memory => ["メモリ", "Memory", "内存", "메모리", "Memoria"],
        // ---- ブックマーク ----
        BookmarksEmpty => [
            "まだありません。ページを開いて Ctrl+D（または ☆）で追加できます。",
            "Nothing here yet. Open a page and press Ctrl+D (or ☆) to add one.",
            "还没有书签。打开页面后按 Ctrl+D（或点 ☆）即可添加。",
            "아직 없습니다. 페이지를 열고 Ctrl+D(또는 ☆)로 추가할 수 있습니다.",
            "Aún no hay ninguno. Abre una página y pulsa Ctrl+D (o ☆) para añadirla."
        ],
        Delete => ["削除", "Delete", "删除", "삭제", "Eliminar"],
        // ---- パスワード ----
        PasswordsNote => [
            "パスワードはこの端末の資格情報ストアにある鍵で暗号化して保存しています。",
            "Passwords are encrypted with a key kept in this device's credential store.",
            "密码使用保存在本设备凭据存储中的密钥加密后保存。",
            "비밀번호는 이 기기의 자격 증명 저장소에 있는 키로 암호화하여 저장합니다.",
            "Las contraseñas se cifran con una clave guardada en el almacén de credenciales de este equipo."
        ],
        PasswordsEmpty => [
            "保存したパスワードはありません。ログインしたとき、保存するか確認します。",
            "No saved passwords. Lite Simple Rust Browser will ask whether to save one when you sign in.",
            "没有已保存的密码。登录时会询问是否保存。",
            "저장된 비밀번호가 없습니다. 로그인할 때 저장할지 물어봅니다.",
            "No hay contraseñas guardadas. Lite Simple Rust Browser preguntará si quieres guardarla al iniciar sesión."
        ],
        Show => ["表示", "Show", "显示", "표시", "Mostrar"],
        Hide => ["隠す", "Hide", "隐藏", "숨기기", "Ocultar"],
        NoUsername => [
            "（ユーザー名なし）",
            "(no username)",
            "（无用户名）",
            "(사용자 이름 없음)",
            "(sin nombre de usuario)"
        ],
        // ---- 設定 ----
        HomePage => ["ホームページ", "Home page", "主页", "홈페이지", "Página de inicio"],
        HomeHint => [
            "空にして保存すると解除します。起動時と、ホームボタン・Alt+Home で開きます。",
            "Save it empty to clear it. It opens at startup, with the Home button and with Alt+Home.",
            "留空并保存即可清除。启动时、点击主页按钮或按 Alt+Home 时打开。",
            "비워 두고 저장하면 해제됩니다. 시작할 때와 홈 버튼, Alt+Home으로 열립니다.",
            "Guárdala vacía para quitarla. Se abre al iniciar, con el botón Inicio y con Alt+Home."
        ],
        Save => ["保存", "Save", "保存", "저장", "Guardar"],
        LoadLevelTitle => [
            "読み込みの負荷（低 → 高）",
            "Load level (low → high)",
            "加载负荷（低 → 高）",
            "로드 부하 (낮음 → 높음)",
            "Nivel de carga (bajo → alto)"
        ],
        LoadLevelNote => [
            "次に読み込むページから効きます。",
            "Takes effect from the next page you load.",
            "从下一个加载的页面开始生效。",
            "다음에 불러오는 페이지부터 적용됩니다.",
            "Se aplica desde la próxima página que cargues."
        ],
        Level1 => ["最軽量", "Minimal", "最轻量", "최소", "Mínimo"],
        Level2 => ["軽量", "Light", "轻量", "가벼움", "Ligero"],
        Level3 => ["標準", "Standard", "标准", "표준", "Estándar"],
        Level4 => ["高負荷", "Heavy", "高负荷", "높은 부하", "Alto"],
        Level5 => ["最大", "Maximum", "最大", "최대", "Máximo"],
        Level1Hint => [
            "HTML と埋め込みの CSS だけ。外部の CSS・画像・スクリプトは取得しません",
            "HTML and inline CSS only. External CSS, images and scripts are not fetched",
            "仅 HTML 和内嵌 CSS，不获取外部 CSS、图片和脚本",
            "HTML과 내장 CSS만 사용합니다. 외부 CSS, 이미지, 스크립트는 가져오지 않습니다",
            "Solo HTML y CSS incrustado. No se descargan CSS externo, imágenes ni scripts"
        ],
        Level2Hint => [
            "外部の CSS まで取得します。画像とスクリプトは使いません",
            "Fetches external CSS too. Images and scripts are not used",
            "同时获取外部 CSS，不使用图片和脚本",
            "외부 CSS까지 가져옵니다. 이미지와 스크립트는 사용하지 않습니다",
            "Descarga también el CSS externo. No se usan imágenes ni scripts"
        ],
        Level3Hint => [
            "画像とスクリプトも使います（既定）",
            "Images and scripts are used too (default)",
            "同时使用图片和脚本（默认）",
            "이미지와 스크립트도 사용합니다 (기본값)",
            "También se usan imágenes y scripts (predeterminado)"
        ],
        Level4Hint => [
            "スクリプトの大きさ・実行時間の予算を 2 倍にします",
            "Doubles the script size and run-time budgets",
            "脚本大小和运行时间的预算加倍",
            "스크립트 크기와 실행 시간 예산을 2배로 늘립니다",
            "Duplica el presupuesto de tamaño y tiempo de ejecución de los scripts"
        ],
        Level5Hint => [
            "予算を 4 倍にし、取得するスクリプトの数も増やします",
            "Quadruples the budgets and allows more scripts to be fetched",
            "预算增至 4 倍，并增加获取的脚本数量",
            "예산을 4배로 늘리고 가져오는 스크립트 수도 늘립니다",
            "Cuadruplica el presupuesto y permite descargar más scripts"
        ],
        ImagesTitle => ["画像", "Images", "图片", "이미지", "Imágenes"],
        ImagesOn => ["読み込む", "Load", "加载", "불러오기", "Cargar"],
        ImagesOff => [
            "読み込まない",
            "Don't load",
            "不加载",
            "불러오지 않기",
            "No cargar"
        ],
        ImagesHint => [
            "「読み込まない」にすると画像を取得せず、通信とメモリを節約します。負荷レベルが 2 以下のときは、この設定にかかわらず読み込みません。",
            "With \"Don't load\", images are not fetched, saving bandwidth and memory. At load level 2 or lower, images are never loaded regardless of this setting.",
            "选择“不加载”后将不获取图片，以节省流量和内存。负荷级别为 2 以下时，无论此设置如何都不加载图片。",
            "'불러오지 않기'를 선택하면 이미지를 가져오지 않아 통신과 메모리를 절약합니다. 부하 수준이 2 이하일 때는 이 설정과 관계없이 불러오지 않습니다.",
            "Con «No cargar» no se descargan las imágenes, lo que ahorra datos y memoria. Con un nivel de carga de 2 o menos, nunca se cargan, sea cual sea este ajuste."
        ],
        PasswordsTitle => [
            "パスワード",
            "Passwords",
            "密码",
            "비밀번호",
            "Contraseñas"
        ],
        PasswordsOn => [
            "保存して入力する",
            "Save and fill in",
            "保存并自动填写",
            "저장하고 자동 입력",
            "Guardar y rellenar"
        ],
        PasswordsOff => ["使わない", "Don't use", "不使用", "사용 안 함", "No usar"],
        PasswordsHint => [
            "ログインしたとき保存を確認し、次からは同じサイトで自動入力します。",
            "Lite Simple Rust Browser asks to save when you sign in, then fills it in next time on the same site.",
            "登录时会询问是否保存，下次在同一网站自动填写。",
            "로그인할 때 저장할지 묻고, 다음부터 같은 사이트에서 자동으로 입력합니다.",
            "Lite Simple Rust Browser pregunta si guardar al iniciar sesión y la rellena la próxima vez en el mismo sitio."
        ],
        PasswordsView => [
            "保存したパスワードを見る",
            "View saved passwords",
            "查看已保存的密码",
            "저장된 비밀번호 보기",
            "Ver contraseñas guardadas"
        ],
        LanguageTitle => ["言語", "Language", "语言", "언어", "Idioma"],
        LanguageHint => [
            "設定・新しいタブ・エラーページなど、Lite Simple Rust Browser 自身の画面の言語です。開いているページの内容は変わりません。",
            "The language of Lite Simple Rust Browser's own screens: settings, new tab, error pages and so on. The content of web pages does not change.",
            "Lite Simple Rust Browser 自身界面的语言，如设置、新标签页和错误页面。网页内容不会改变。",
            "설정, 새 탭, 오류 페이지 등 Lite Simple Rust Browser 자체 화면의 언어입니다. 웹 페이지의 내용은 바뀌지 않습니다.",
            "El idioma de las pantallas de Lite Simple Rust Browser: ajustes, nueva pestaña, páginas de error, etc. El contenido de las páginas web no cambia."
        ],
        // ---- メモリ ----
        MemoryTitle => ["メモリ", "Memory", "内存", "메모리", "Memoria"],
        Usage => ["使用量", "Usage", "使用量", "사용량", "Uso"],
        BrowserProcess => [
            "Browser プロセス",
            "Browser process",
            "Browser 进程",
            "Browser 프로세스",
            "Proceso Browser"
        ],
        ContentProcess => [
            "Content プロセス",
            "Content process",
            "Content 进程",
            "Content 프로세스",
            "Proceso Content"
        ],
        Total => [
            "合計（予算の対象）",
            "Total (counted against the budget)",
            "合计（计入预算）",
            "합계 (예산 대상)",
            "Total (cuenta para el presupuesto)"
        ],
        Budget => ["予算", "Budget", "预算", "예산", "Presupuesto"],
        Pressure => [
            "OS のメモリ圧迫",
            "OS memory pressure",
            "系统内存压力",
            "OS 메모리 압박",
            "Presión de memoria del SO"
        ],
        PressureNone => ["なし", "None", "无", "없음", "Ninguna"],
        PressureModerate => [
            "中（背景のタブを凍結）",
            "Moderate (freezing background tabs)",
            "中（冻结后台标签页）",
            "보통 (백그라운드 탭 동결)",
            "Moderada (se congelan pestañas en segundo plano)"
        ],
        PressureCritical => [
            "危険（背景のタブを破棄）",
            "Critical (discarding background tabs)",
            "严重（丢弃后台标签页）",
            "위험 (백그라운드 탭 폐기)",
            "Crítica (se descartan pestañas en segundo plano)"
        ],
        Policy => ["方針", "Policy", "策略", "정책", "Política"],
        FreezeAfter => [
            "背景に回って凍結するまで",
            "Until a background tab is frozen",
            "转入后台到冻结的时间",
            "백그라운드로 가서 동결되기까지",
            "Hasta congelar una pestaña en segundo plano"
        ],
        DiscardAfter => [
            "見なくなって破棄するまで",
            "Until an unused tab is discarded",
            "不再查看到丢弃的时间",
            "보지 않게 된 후 폐기되기까지",
            "Hasta descartar una pestaña sin usar"
        ],
        Demotions => [
            "これまでの降格",
            "Demotions so far",
            "累计降级",
            "지금까지의 강등",
            "Degradaciones hasta ahora"
        ],
        DemotionCount => [
            "凍結 {0} 回 / 破棄 {1} 回",
            "Frozen {0} / discarded {1}",
            "冻结 {0} 次 / 丢弃 {1} 次",
            "동결 {0}회 / 폐기 {1}회",
            "Congeladas {0} / descartadas {1}"
        ],
        Seconds => ["{0} 秒", "{0} s", "{0} 秒", "{0}초", "{0} s"],
        SecondsAgo => [
            "{0} 秒前",
            "{0} s ago",
            "{0} 秒前",
            "{0}초 전",
            "hace {0} s"
        ],
        TabsTitle => ["タブ", "Tabs", "标签页", "탭", "Pestañas"],
        Showing => [
            "（表示中）",
            " (showing)",
            "（当前显示）",
            " (표시 중)",
            " (visible)"
        ],
    }
}

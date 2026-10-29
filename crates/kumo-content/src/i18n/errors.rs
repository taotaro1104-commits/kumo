//! 読み込めなかったときのページ（エラーページ）の文言。

messages! {
    Er {
        PageTitle => [
            "読み込めません",
            "Can't load the page",
            "无法加载",
            "불러올 수 없음",
            "No se puede cargar"
        ],
        Reload => ["再読み込み", "Reload", "刷新", "새로 고침", "Recargar"],
        Back => ["戻る", "Back", "后退", "뒤로", "Atrás"],
        Detail => ["詳細: {0}", "Details: {0}", "详情：{0}", "자세히: {0}", "Detalles: {0}"],
        BlockedHead => [
            "このページはブロックされました",
            "This page was blocked",
            "此页面已被拦截",
            "이 페이지가 차단되었습니다",
            "Esta página se ha bloqueado"
        ],
        BlockedHint => [
            "コンテンツブロックのフィルタに一致したため、読み込みませんでした。",
            "It matched a content-blocking filter, so it was not loaded.",
            "由于匹配了内容拦截过滤规则，未加载该页面。",
            "콘텐츠 차단 필터와 일치하여 불러오지 않았습니다.",
            "Coincide con un filtro de bloqueo de contenido, por lo que no se ha cargado."
        ],
        BadUrlHead => [
            "URL が正しくありません",
            "The URL is not valid",
            "网址无效",
            "URL이 올바르지 않습니다",
            "La URL no es válida"
        ],
        BadUrlHint => [
            "アドレスの綴りを確認してください。",
            "Check the spelling of the address.",
            "请检查地址的拼写。",
            "주소의 철자를 확인하세요.",
            "Comprueba que la dirección esté bien escrita."
        ],
        SchemeHead => [
            "この種類のアドレスは開けません",
            "This kind of address can't be opened",
            "无法打开此类地址",
            "이 종류의 주소는 열 수 없습니다",
            "No se puede abrir este tipo de dirección"
        ],
        SchemeHint => [
            "Lite Simple Rust Browser が開けるのは http・https・file のページです。",
            "Lite Simple Rust Browser can only open http, https and file pages.",
            "Lite Simple Rust Browser 只能打开 http、https 和 file 页面。",
            "Kumo는 http, https, file 페이지만 열 수 있습니다.",
            "Lite Simple Rust Browser solo puede abrir páginas http, https y file."
        ],
        TooLargeHead => [
            "ページが大きすぎます",
            "The page is too large",
            "页面过大",
            "페이지가 너무 큽니다",
            "La página es demasiado grande"
        ],
        TooLargeHint => [
            "32MB を超えるページは開けません。",
            "Pages over 32 MB can't be opened.",
            "无法打开超过 32MB 的页面。",
            "32MB를 넘는 페이지는 열 수 없습니다.",
            "No se pueden abrir páginas de más de 32 MB."
        ],
        DisconnectedHead => [
            "通信を担当するプロセスが止まりました",
            "The network process has stopped",
            "网络进程已停止",
            "네트워크 프로세스가 중지되었습니다",
            "El proceso de red se ha detenido"
        ],
        DisconnectedHint => [
            "ブラウザを起動し直してください。",
            "Please restart the browser.",
            "请重新启动浏览器。",
            "브라우저를 다시 시작하세요.",
            "Reinicia el navegador."
        ],
        FileMissingHead => [
            "ファイルが見つかりません",
            "File not found",
            "找不到文件",
            "파일을 찾을 수 없습니다",
            "No se encuentra el archivo"
        ],
        FileMissingHint => [
            "パスが正しいか、ファイルが移動・削除されていないか確認してください。",
            "Check that the path is correct and the file has not been moved or deleted.",
            "请检查路径是否正确，文件是否已被移动或删除。",
            "경로가 올바른지, 파일이 이동되거나 삭제되지 않았는지 확인하세요.",
            "Comprueba que la ruta sea correcta y que el archivo no se haya movido ni eliminado."
        ],
        HostMissingHead => [
            "サーバーが見つかりません",
            "Server not found",
            "找不到服务器",
            "서버를 찾을 수 없습니다",
            "No se encuentra el servidor"
        ],
        HostMissingHint => [
            "アドレスの綴りと、インターネットへの接続を確認してください。",
            "Check the spelling of the address and your internet connection.",
            "请检查地址的拼写和网络连接。",
            "주소의 철자와 인터넷 연결을 확인하세요.",
            "Comprueba la dirección y tu conexión a internet."
        ],
        TimeoutHead => [
            "応答がありません",
            "The server took too long to respond",
            "没有响应",
            "응답이 없습니다",
            "El servidor no responde"
        ],
        TimeoutHint => [
            "サーバーが混んでいる可能性があります。時間をおいて再読み込みしてください。",
            "The server may be busy. Wait a moment and reload.",
            "服务器可能繁忙，请稍后刷新。",
            "서버가 혼잡할 수 있습니다. 잠시 후 다시 불러오세요.",
            "Puede que el servidor esté saturado. Espera un momento y recarga."
        ],
        RefusedHead => [
            "接続が拒否されました",
            "The connection was refused",
            "连接被拒绝",
            "연결이 거부되었습니다",
            "Conexión rechazada"
        ],
        RefusedHint => [
            "サーバーが動いていないか、接続を受け付けていません。",
            "The server is not running or is not accepting connections.",
            "服务器未运行，或不接受连接。",
            "서버가 실행 중이 아니거나 연결을 받지 않습니다.",
            "El servidor no está en marcha o no acepta conexiones."
        ],
        TlsHead => [
            "安全な接続を確立できません",
            "Can't establish a secure connection",
            "无法建立安全连接",
            "보안 연결을 만들 수 없습니다",
            "No se puede establecer una conexión segura"
        ],
        TlsHint => [
            "サーバーの証明書に問題があるため、安全のため読み込みを中止しました。",
            "The server's certificate has a problem, so loading was stopped for your safety.",
            "服务器证书存在问题，为了安全已停止加载。",
            "서버 인증서에 문제가 있어 안전을 위해 불러오기를 중단했습니다.",
            "El certificado del servidor tiene un problema, por lo que se ha detenido la carga por seguridad."
        ],
        OtherHead => [
            "ページを読み込めませんでした",
            "The page could not be loaded",
            "无法加载页面",
            "페이지를 불러오지 못했습니다",
            "No se ha podido cargar la página"
        ],
        OtherHint => [
            "インターネットへの接続を確認して、再読み込みしてください。",
            "Check your internet connection and reload.",
            "请检查网络连接后刷新。",
            "인터넷 연결을 확인한 후 다시 불러오세요.",
            "Comprueba tu conexión a internet y recarga."
        ],
    }
}

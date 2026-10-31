---
title: "watchtower-rs"
site_name: "watchtower-rs"
description: "Un vigilante de páginas web compatible con changedetection.io, hecho en Rust. Un solo binario y unos 24 MB para 5.000 vigilancias."
tagline: "Vigila los cambios de las páginas en silencio."
lead: "watchtower-rs revisa páginas web de forma periódica y te avisa de las diferencias. Tiene una REST API compatible con changedetection.io y funciona como un único binario de Rust."
pills: ["Un solo binario", "SQLite", "Compatible con changedetection.io", "REST API"]
nav_all: "Todas las apps"
nav_features: "Características"
nav_start: "Primeros pasos"
nav_status: "Estado"
stats_title: "Uso de memoria (medido en una compilación release de Windows)"
stats:
  - { value: "~17 MB", label: "Con 1.000 vigilancias registradas en reposo" }
  - { value: "~24 MB", label: "Pico al vigilar 5.000 páginas cada 300 s" }
  - { value: "~90 s", label: "Para la primera descarga de las 5.000 páginas" }
stats_note: "El ejecutable pesa unos 28 MB. Se midió con un sitio simulado propio; los resultados cambian según el entorno."
features_title: "Características"
features:
  - { icon: "🍃", title: "Poca memoria", body: "La memoria crece despacio al añadir vigilancias. Las instantáneas se comprimen y se guardan en archivos." }
  - { icon: "🔁", title: "Compatible", body: "La REST API sigue la forma original y se autentica con x-api-key. También puedes importar el url-watches.json original." }
  - { icon: "✂️", title: "Compara solo lo importante", body: "Acota el objetivo con selectores CSS, XPath, JSONPath o jq y quita el ruido con expresiones regulares y texto ignorado. Si quieres, avisa solo cuando aparezca un texto desencadenante." }
  - { icon: "🌐", title: "También páginas con JavaScript", body: "Se conecta a un Chrome que ejecutas aparte y obtiene la página renderizada. Registra pasos de clic, escritura y espera, o elige un elemento sobre una captura con el selector visual." }
  - { icon: "🏷️", title: "Precio, stock y PDF", body: "Lee precios de los datos estructurados de una página de producto y avisa al llegar al umbral. También compara el texto de los PDF." }
  - { icon: "🔔", title: "Muchos destinos de aviso", body: "Discord, Slack, Telegram, ntfy, Gotify, Mattermost, Pushover, correo y webhooks; el resto se reenvía a una Apprise API. Título y cuerpo se pueden personalizar con plantillas." }
  - { icon: "🛠️", title: "Control fino de la descarga", body: "Define cabeceras, método, cuerpo, proxy e intervalo por vigilancia, por etiqueta o de forma global. Gana el ajuste más cercano." }
  - { icon: "🛡️", title: "Pensado para la seguridad", body: "Protección SSRF contra direcciones internas, contraseñas con argon2, protección CSRF y lista de nombres de Host permitidos. Los valores de cabeceras y los secretos de avisos no llegan a los registros." }
  - { icon: "📈", title: "Fácil de operar", body: "Documentación OpenAPI, /metrics en formato Prometheus, feed RSS, copia de seguridad y restauración, y comprobación de salud. Incluye archivos de Docker." }
how_title: "Cómo funciona"
how:
  - { title: "Descargar", body: "Las páginas se descargan con el intervalo que elijas. Las conexiones a direcciones internas se rechazan por defecto." }
  - { title: "Ordenar", body: "Se acota el objetivo, se quitan elementos y textos cambiantes y el resultado pasa a texto plano." }
  - { title: "Comparar y avisar", body: "Se guardan las diferencias con la última revisión y solo se envía un aviso si se cumplen tus condiciones. Los avisos fallidos se reintentan." }
start_title: "Primeros pasos"
start_lead: "Define una contraseña y arráncalo desde la línea de comandos. La interfaz de administración se abre por defecto en 127.0.0.1:5000."
start_code: |-
  watchtower set-password     # contraseña de la interfaz (una línea por la entrada estándar)
  watchtower api-key          # genera una clave de la REST API
  watchtower watch add https://example.com/ --interval 300
  watchtower run              # inicia las revisiones periódicas y la interfaz
start_note: "Se configura con variables de entorno (WATCHTOWER_DATA_DIR, WATCHTOWER_LISTEN, WATCHTOWER_BASE_URL y más) o un archivo de configuración."
status_title: "Estado"
status_body: "En desarrollo. Aún no se publican descargas ni el código fuente. Se ha probado en Windows 11."
todo_title: "Aún sin verificar"
todo:
  - "Construir y ejecutar la imagen de Docker"
  - "Un binario estático de Linux (musl) y aarch64"
  - "Enviar a través de un servidor de correo real con TLS o STARTTLS"
footer: "© watchtower-rs — Apache-2.0"
---

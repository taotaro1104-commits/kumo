---
title: "Lite Simple Rust Browser"
description: "Un navegador web ligero y sencillo, escrito desde cero en Rust con el bajo consumo de memoria como primer objetivo."
tagline: "Ligero y sencillo. Un navegador web hecho en Rust."
lead: "Analiza HTML y CSS, calcula el diseño, dibuja, se comunica por la red y ejecuta JavaScript con su propio motor, así que sigue siendo ligero por muchas pestañas que abras."
nav_features: "Funciones"
nav_get: "Descargar"
cta_source: "Código fuente"
ad_label: "Publicidad"
stats_title: "Uso de memoria (medido en una compilación release)"
stats:
  - { mb: 22, value: "~22 MB", label: "Pestaña en blanco" }
  - { mb: 65, value: "~65 MB", label: "Un artículo de Wikipedia" }
  - { mb: 185, value: "~185 MB", label: "Diez pestañas" }
features_title: "Funciones"
features:
  - { icon: "🍃", title: "Poca memoria", body: "Las pestañas en segundo plano se congelan o descartan automáticamente para mantenerse dentro del presupuesto de memoria." }
  - { icon: "🔀", title: "Tres procesos", body: "Browser, Content y Network se ejecutan por separado. La ventana responde aunque la página sea pesada, y Content se reinicia solo si se detiene." }
  - { icon: "⚡", title: "JavaScript", body: "Ejecuta DOM, eventos, temporizadores, fetch y más con el motor Boa." }
  - { icon: "🎚️", title: "Niveles de carga 1-5", body: "De «Mínimo» a «Máximo». La carga de imágenes se puede activar o desactivar por separado." }
  - { icon: "🔑", title: "Gestor de contraseñas", body: "Pregunta si quieres guardar al iniciar sesión y las rellena la próxima vez en el mismo sitio. Las contraseñas se cifran y la clave se guarda en el Administrador de credenciales de Windows." }
  - { icon: "⭐", title: "Marcadores", body: "Ctrl+D para añadir, Ctrl+B para ver la lista. Elige tu página de inicio en Ajustes." }
  - { icon: "🕘", title: "Historial y descargas", body: "Ctrl+H para el historial, Ctrl+J para las descargas. Los enlaces a archivos se guardan sin salir de la página y con el progreso a la vista." }
  - { icon: "🔍", title: "Zoom de página", body: "Ctrl+más, Ctrl+menos o Ctrl+rueda, del 25 % al 500 %. Ctrl+0 lo restablece." }
  - { icon: "あ", title: "Entrada en japonés", body: "El texto convertido con un IME se escribe en la barra de direcciones y en los campos de la página." }
  - { icon: "🌏", title: "Cinco idiomas", body: "Japonés, inglés, chino simplificado, coreano y español." }
  - { icon: "🛡️", title: "Bloqueo de contenido", body: "Bloquea anuncios y otras peticiones no deseadas con listas de filtros." }
start_title: "Primeros pasos"
start_lead: "Al iniciar se abre una pestaña nueva. Escribe una URL o una búsqueda en el cuadro superior."
keys:
  - { k: "Ctrl L", d: "Ir a la barra de direcciones" }
  - { k: "Ctrl T", d: "Pestaña nueva" }
  - { k: "Ctrl W", d: "Cerrar pestaña" }
  - { k: "Ctrl D", d: "Añadir a marcadores" }
  - { k: "Ctrl H", d: "Historial" }
  - { k: "Ctrl J", d: "Descargas" }
  - { k: "Ctrl +/-", d: "Acercar o alejar" }
start_note: "Los ajustes están en about:settings, las contraseñas en about:passwords y la memoria en about:memory."
download_title: "Descargar"
download_button: "Descargar desde Google Drive"
recommended: "Recomendado"
sha_summary: "Ver sumas de comprobación (SHA-256)"
download_body: "Para Windows 11 (64 bits). Ejecuta el instalador o simplemente descomprime el zip. El historial y los ajustes se guardan en %LOCALAPPDATA%\\Kumo. Licencia a tu elección: MIT o Apache-2.0."
release_title: "Última versión"
release_version: "Versión 0.1.0 (2026-10-29)"
release_files:
  - { name: "kumo-0.1.0-windows-x64-setup.exe", kind: "Instalador", size: "8.8 MB", sha: "2f2a017fdaf6d8f85c7e70e497facdd4a00bfc2f6081c84974105158da1d1e84" }
  - { name: "kumo-0.1.0-windows-x64.zip", kind: "Zip (descomprime y usa)", size: "10.4 MB", sha: "d78c7542a9a61225636428c30e1b2fc8d30df193cc19554963bbc030317bdba8" }
sha_note: "Compara el SHA-256 del archivo descargado con los valores siguientes."
changes_title: "Cambios recientes"
changes:
  - "Zoom de página (Ctrl+más, Ctrl+menos, Ctrl+rueda, Ctrl+0)"
  - "Compatibilidad con la entrada en japonés (IME), con el texto en composición en la barra de direcciones"
  - "Historial (Ctrl+H) y descargas (Ctrl+J). Los enlaces a archivos se guardan sin salir de la página"
  - "Nuevo nombre, Lite Simple Rust Browser, y un icono original"
  - "Gestor de contraseñas y cinco idiomas de interfaz"
limits_title: "Limitaciones actuales"
limits_body: "Está en desarrollo, así que no todos los sitios se muestran perfectamente. Aún no admite módulos ES, iframes, canvas, renderizado por GPU ni SVG en línea."
shop_title: "Recomendado"
shop_lead: "Busca ordenadores, accesorios y artículos cotidianos en las tiendas japonesas siguientes."
rakuten_label: "Buscar en Rakuten Ichiba"
amazon_label: "Buscar en Amazon.co.jp"
shop_disclosure: "Este sitio utiliza publicidad (Google AdSense) y programas de afiliados (Rakuten Affiliate y Amazon Associates). Como afiliado de Amazon, este sitio obtiene ingresos por las compras adscritas que cumplen los requisitos aplicables."
---

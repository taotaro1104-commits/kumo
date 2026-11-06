---
title: "LocalMind"
site_name: "LocalMind"
description: "Una app de Android escrita en Rust que transcribe grabaciones, redacta actas de reuniones, busca en tus documentos y responde preguntas con IA, todo en el propio dispositivo. Tus grabaciones y documentos no se envían fuera."
tagline: "Reuniones y documentos, en tu dispositivo."
lead: "LocalMind transcribe grabaciones, redacta actas de reuniones, busca en tus propios documentos y responde a tus preguntas con IA, todo dentro de tu teléfono. Las grabaciones, las transcripciones y los documentos nunca salen del dispositivo."
pills: ["Android", "Funciona sin conexión", "Rust", "Pantallas en japonés e inglés"]
nav_all: "Todas las apps"
nav_features: "Características"
nav_how: "Cómo se usa"
nav_get: "Cómo obtenerla"
nav_status: "Estado"
stats_title: "Qué hace"
stats:
  - { value: "3 funciones", label: "Actas de reuniones, búsqueda de documentos y preguntas a la IA" }
  - { value: "En el dispositivo", label: "Grabaciones, transcripciones y documentos no se envían fuera" }
  - { value: "3 modelos", label: "Elige el modelo de IA por tamaño y cámbialo después" }
stats_note: "Los modelos de IA (de 0,5 a 2,1 GB aproximadamente) no vienen incluidos en la app. Se descarga uno la primera vez que lo necesitas, con tu consentimiento. Un modelo pequeño es más rápido, pero las actas y las respuestas son menos precisas."
shots_title: "Capturas de pantalla"
shots_note: "Las pantallas se capturaron en un emulador de Android 15 (pantallas en japonés), con documentos de ejemplo y un breve audio de una reunión simulada."
shots:
  - { img: "search.jpg", caption: "Búsqueda de documentos, con fuentes" }
  - { img: "groups.jpg", caption: "Grupos de documentos" }
  - { img: "minutes.jpg", caption: "Acta, con la hora de la intervención citada" }
  - { img: "ask.jpg", caption: "Preguntar a la IA" }
  - { img: "model.jpg", caption: "Descarga de un modelo de IA" }
features_title: "Características"
features:
  - { icon: "🎙️", title: "Graba y transcribe", body: "Graba reuniones y clases solo en el dispositivo. Antes de grabar puedes confirmar el consentimiento de los participantes, y durante la grabación siempre se muestra una notificación. La transcripción se hace en el dispositivo (whisper.cpp)." }
  - { icon: "📝", title: "Actas con fuentes", body: "A partir de la transcripción redacta un resumen, las decisiones, las tareas y los asuntos sin resolver. Cada punto indica la hora de la intervención en que se basa, y los puntos cuya fuente no se puede confirmar se omiten." }
  - { icon: "🔎", title: "Búsqueda de documentos en japonés", body: "Registra documentos de texto o Markdown y búscalos, incluso con una pregunta reformulada. Muestra dónde está la respuesta, con el documento, el encabezado y el número de línea, y responde «no se ha encontrado» si no hay respuesta." }
  - { icon: "🌐", title: "Importar un documento desde una URL", body: "Escribe la URL de una página y se toma solo su texto principal como documento. Solo obtiene páginas https públicas y nunca lee tu propio dispositivo ni una red interna." }
  - { icon: "🗂️", title: "Grupos de documentos", body: "Reúne los documentos en grupos y cambia la lista entre ellos. Puedes limitar la búsqueda al grupo elegido, mover documentos de un grupo a otro o borrar un grupo entero de una vez." }
  - { icon: "💬", title: "Pregunta a la IA", body: "Haz preguntas a la IA sin cargar ningún documento. Responde solo en el dispositivo o, únicamente si lo eliges, busca antes en Wikipedia. Te mostramos las palabras de búsqueda antes de enviarlas." }
  - { icon: "🔁", title: "Elige y cambia de modelo de IA", body: "Elige entre Qwen2.5 0.5B, 1.5B y 3B, o indica la URL de otro modelo (GGUF). Los modelos descargados se conservan y puedes cambiar entre ellos o borrarlos cuando quieras." }
  - { icon: "🌍", title: "Pantallas en japonés e inglés", body: "Las pantallas de la app están en japonés e inglés y siguen el idioma del dispositivo (en Android 13 y posteriores también puedes elegirlo por app en los Ajustes). La IA responde en el mismo idioma en que se le pregunta." }
privacy_title: "Cómo se tratan tus datos"
privacy_body: "Las grabaciones, las transcripciones y los documentos no se envían fuera. La app usa la red solo en tres casos, y en todos ellos eres tú quien lo decide."
privacy:
  - "Descargar un modelo de IA (la primera vez que lo usas, después de comprobar el origen)"
  - "Elegir «buscar en Wikipedia» al preguntar a la IA (solo se envían las palabras de búsqueda breves que creó la IA, tras tu confirmación; no se envían tu pregunta completa ni tus documentos)"
  - "Usar «Añadir desde URL» en un documento (se conecta una sola vez, a la URL que escribiste)"
how_title: "Cómo se usa"
how:
  - { title: "Consigue un modelo de IA", body: "La primera vez que hagas un acta o una pregunta, elige un modelo (o escribe una URL) y descárgalo. Se recomienda usar Wi-Fi." }
  - { title: "Graba y haz el acta", body: "Graba en la pantalla de grabación, detén la grabación y ejecuta la transcripción. Después pulsa «Hacer acta» en la pantalla de resultados y en unos minutos tendrás el acta. Puedes compartirla." }
  - { title: "Añade documentos, busca o pregunta", body: "En la pantalla de búsqueda, añade documentos desde texto, un archivo o una URL, y escribe una pregunta para ver los pasajes que la responden. Para preguntar a la IA sin documentos, usa la pantalla de preguntas." }
download_title: "Cómo obtenerla"
download_body: "El APK de Android (la app en sí) se comparte en Google Drive. El código fuente está en GitHub y también puedes compilarla tú mismo."
download_note: "El APK está firmado con una clave que generamos nosotros, así que tu dispositivo te pedirá permitir apps de orígenes desconocidos. Ocupa unos 700 MB, por lo que se recomienda descargarlo con Wi-Fi. Los modelos de IA no están incluidos; se descarga uno aparte la primera vez que lo usas."
download_button: "Descargar desde Google Drive"
download_url: "https://drive.google.com/drive/folders/1hfZGFiTYGIdArpSbNyXrt-hWH11323Wv?usp=drive_link"
android_file: "LocalMind-0.1.apk"
android_sha: "078f7c82afc78e46455492537aa14018be75717d00c0895ee2696f5b1c668224"
sha_label: "SHA-256 (v0.1)"
cta_source: "Código fuente (GitHub)"
repo_url: "https://github.com/taotaro1104-commits/localmind"
status_title: "Estado"
status_body: "En desarrollo (v0.1, prototipo). En un emulador de Android 15 hemos comprobado todo el flujo desde la grabación hasta el acta, la búsqueda e importación de documentos y las preguntas a la IA. Las respuestas de la IA pueden ser erróneas, así que comprueba tú mismo lo importante."
todo_title: "Aún sin comprobar o no admitido"
todo:
  - "Dispositivos reales y su velocidad (solo se ha probado en un emulador; aún no se conoce la velocidad en un dispositivo arm64)"
  - "La precisión de los modelos pequeños (la clasificación de las actas y las respuestas pueden ser todavía toscas)"
  - "Reuniones largas (la transcripción usada para el acta está limitada a 5.000 caracteres)"
  - "No se admite la importación de PDF ni la lectura de texto en imágenes (OCR)"
  - "Distribución en Google Play (la app ocupa unos 700 MB, por encima del límite de tamaño)"
  - "Transcripción y actas de reuniones en inglés (por ahora la transcripción y las actas están pensadas para el japonés)"
footer: "© LocalMind"
---

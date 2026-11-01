---
title: "RustHighBookViewer"
site_name: "RustHighBookViewer"
description: "Un visor de cómics e imágenes escrito en Rust. Lee ZIP/CBZ, 7z, RAR, EPUB y carpetas de imágenes, con doble página de derecha a izquierda y más. Para Android y Windows."
tagline: "Abre un libro y empieza a leer."
lead: "RustHighBookViewer abre archivos comprimidos y carpetas llenos de cómics o imágenes. Admite doble página de derecha a izquierda, zoom, ajuste de color y recuerda por dónde ibas, en Android y Windows."
pills: ["Android", "Windows", "ZIP / CBZ / 7z / RAR / EPUB", "De derecha a izquierda"]
nav_all: "Todas las apps"
nav_features: "Características"
nav_how: "Cómo se usa"
nav_get: "Descarga"
nav_status: "Estado"
stats_title: "Qué admite"
stats:
  - { value: "5 tipos", label: "Abre ZIP/CBZ, 7z, RAR, EPUB y carpetas de imágenes" }
  - { value: "4 vistas", label: "Una página, doble página, doble automática y desplazamiento vertical" }
  - { value: "2 plataformas", label: "Android 8.0 o superior y Windows" }
stats_note: "Admite RAR 4.x / 5.x (sin cifrar). Los archivos 7z y RAR se descomprimen en el almacenamiento del dispositivo al abrirlos. EPUB admite libros con una imagen por página (cómics, libros de fotos)."
features_title: "Características"
features:
  - { icon: "📖", title: "De derecha a izquierda o al revés", body: "Cambia el sentido de encuadernación y el orden de la doble página se ajusta. La portada puede ir sola y las páginas apaisadas se muestran de una en una." }
  - { icon: "🔍", title: "Zoom y lupa", body: "Pellizca o toca dos veces para ampliar y desplazarte. La versión de Android tiene además una lupa que amplía 2x lo que recorres con el dedo." }
  - { icon: "🗂️", title: "Cuadrícula de páginas", body: "Salta a cualquier página desde las miniaturas de todo el libro (versión de Android)." }
  - { icon: "🎨", title: "Ajuste de color", body: "Brillo, contraste, gamma, blanco y negro, sepia e invertir. Los ajustes se recuerdan por libro (versión de Android)." }
  - { icon: "⏱️", title: "Paso automático", body: "Pasa las páginas solo cada 3, 5, 10 o 20 segundos y se detiene en la última (versión de Android)." }
  - { icon: "📚", title: "Estantería y seguir leyendo", body: "Una estantería con historial, favoritos y búsqueda por título. Al cerrar un libro se guarda tu posición, así que puedes continuar donde lo dejaste." }
  - { icon: "📁", title: "También carpetas", body: "Elige una carpeta: las imágenes de cada carpeta forman un libro y cada archivo EPUB, ZIP, 7z o RAR es otro libro, subcarpetas incluidas, y todo se añade a la estantería de una vez. En Android, al permitir «Acceso a todos los archivos» puedes elegir carpetas como Download directamente." }
  - { icon: "🛡️", title: "Pensado para la seguridad", body: "Hay límites para el tamaño descomprimido y el número de archivos, y la descompresión se detiene si queda poco espacio libre. Las rutas como ../ dentro de un archivo se anulan." }
  - { icon: "🔢", title: "Ir a una página y marcadores", body: "Escribe un número de página para saltar, o salta desde la lista de marcadores. Añade o quita un marcador con un toque (versión de Android)." }
  - { icon: "⚙️", title: "Ajustes", body: "Elige los valores por defecto de la vista, el sentido de encuadernación, el ajuste, el cambio de página con las teclas de volumen, la pantalla encendida y el orden de la estantería, y cuánta memoria puede usar la caché de imágenes (versión de Android)." }
  - { icon: "📘", title: "Cómics en EPUB", body: "Abre EPUB con una imagen por página, en orden de lectura. No admite EPUB de texto." }
  - { icon: "⚡", title: "Más rápido la segunda vez", body: "Recuerda el tamaño de las páginas, así que las carpetas y archivos se abren más rápido la próxima vez." }
how_title: "Cómo se usa"
how:
  - { title: "Abrir", body: "En Android, elige \"Archivo\" o \"Carpeta\" en la estantería. En Windows, arrastra un archivo o una carpeta sobre el exe." }
  - { title: "Leer", body: "Toca el lado izquierdo o derecho de la pantalla para pasar de página. Toca el centro para ver un menú donde cambiar la vista y el sentido de encuadernación." }
  - { title: "Continuar", body: "Al cerrar un libro se guarda tu posición. Elígelo en la estantería para seguir." }
download_title: "Descarga"
download_button: "Descargar desde Google Drive"
download_body: "El APK de Android y el exe de Windows se comparten en Google Drive. No hay instalador; basta con ejecutar el exe."
download_note: "El APK está firmado con una clave que generamos nosotros, así que el dispositivo te pedirá permitir apps de orígenes desconocidos. El exe no está firmado y Windows SmartScreen puede mostrar una advertencia."
cta_source: "Código fuente"
sha_label: "SHA-256 (v0.1.3)"
release_label: "Notas de versión y código (GitHub)"
download_url: "https://drive.google.com/drive/folders/15qKnVY5FglIUeVdFlwyLt3eVNU_XtltK?usp=sharing"
repo_url: "https://github.com/taotaro1104-commits/RustHighBookViewer"
release_url: "https://github.com/taotaro1104-commits/RustHighBookViewer/releases"
android_file: "RustHighBookViewer-0.1.3-android.apk"
windows_file: "RustHighBookViewer-desktop.exe"
android_sha: "2bef5262f6cbb348b7961bee0426102f33318eae24f88c17bb6ad4bf15d6d5fc"
windows_sha: "5211618dec0cff0fc5f1179d5a7de8f4f4556fc6baadc59ac90c4b095672854a"
status_title: "Estado"
status_body: "En desarrollo (v0.1.3, versión preliminar). En Android hemos comprobado, en un emulador de Android 15, que 7z, RAR, EPUB y la importación de carpetas completas funcionan. El código fuente está en GitHub."
todo_title: "Aún sin verificar o no admitido"
todo:
  - "Android en un dispositivo real (solo comprobado en emulador)"
  - "El aspecto y el manejo de la versión de Windows (solo se comprobó que arranca y sigue funcionando)"
  - "Archivos con contraseña y RAR divididos en varios archivos (no admitidos)"
footer: "© RustHighBookViewer — MIT License"
---

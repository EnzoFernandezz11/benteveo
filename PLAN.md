# Plan de desarrollo — aplicación de enfoque para Linux

## 1. Objetivo

Construir una aplicación de escritorio nativa para Linux, escrita en Rust, que reúna tres herramientas de concentración en una interfaz mínima inspirada en una terminal:

1. Generador de ruido para concentración.
2. Temporizador Pomodoro configurable.
3. Checklist de tareas con persistencia local.

La aplicación debe ser liviana, funcionar sin conexión, guardar su estado localmente y distribuirse inicialmente como un ejecutable de Linux.

## 2. Alcance del MVP

### Incluido

- Una sola ventana con tres pestañas: `Ruido`, `Pomodoro` y `Tareas`.
- Tema oscuro, tipografía monoespaciada y navegación simple mediante ratón o teclado.
- Ruido blanco, rosa y marrón generado o reproducido de forma continua.
- Control de reproducción y volumen.
- Dos modos Pomodoro predeterminados:
  - Clásico: 25 minutos de trabajo y 5 minutos de descanso.
  - Profundo: 90 minutos de trabajo y 10 minutos de descanso.
- Modo Pomodoro personalizado, con tiempos de trabajo y descanso editables.
- Inicio, pausa, reanudación, salto de etapa y reinicio del temporizador.
- Notificación de escritorio y aviso sonoro al terminar una etapa.
- Checklist para crear, editar, completar, reordenar y eliminar tareas.
- Guardado automático de preferencias, tareas y estado relevante.
- Atajos de teclado básicos.

### Fuera del MVP

- Sincronización en la nube.
- Cuentas de usuario.
- Aplicaciones para otros sistemas operativos.
- Estadísticas complejas o gamificación.
- Integraciones con calendarios o gestores externos.
- Tienda de sonidos o descarga de audio desde Internet.

## 3. Decisiones técnicas propuestas

- Lenguaje: Rust estable.
- Interfaz: `egui` con `eframe`, por su sencillez, bajo costo de desarrollo y facilidad para crear una aplicación nativa de una sola ventana.
- Audio: `rodio` o una abstracción equivalente compatible con Linux.
- Persistencia: archivos locales serializados con `serde` y formato JSON.
- Directorio de datos: respetar los directorios estándar de Linux mediante una biblioteca como `directories`.
- Notificaciones: integración con las notificaciones de escritorio de Linux mediante una biblioteca compatible con D-Bus.
- Errores y registro: errores tipados para la lógica y logging sencillo para diagnóstico.

Evitar fijar versiones manualmente en este documento. El agente debe seleccionar versiones estables y compatibles al crear el proyecto y conservarlas en `Cargo.lock`.

## 4. Arquitectura

Mantener separadas la interfaz, la lógica y la infraestructura para que el temporizador y la checklist puedan probarse sin abrir una ventana ni reproducir audio.

```text
src/
├── main.rs              # Inicio de la aplicación
├── app.rs               # Estado global y navegación
├── ui/
│   ├── mod.rs
│   ├── noise_view.rs
│   ├── pomodoro_view.rs
│   ├── tasks_view.rs
│   └── theme.rs
├── domain/
│   ├── mod.rs
│   ├── pomodoro.rs       # Máquina de estados del temporizador
│   └── task.rs           # Modelo y operaciones de tareas
├── services/
│   ├── mod.rs
│   ├── audio.rs
│   ├── notifications.rs
│   └── persistence.rs
└── config.rs             # Configuración persistente
```

Principios:

- La UI no debe contener lógica temporal crítica.
- El temporizador debe calcular el tiempo restante a partir de instantes reales, no contando repintados de pantalla.
- Audio y notificaciones deben exponerse mediante interfaces pequeñas y reemplazables en pruebas.
- La persistencia debe realizar escrituras atómicas para reducir el riesgo de archivos corruptos.
- Un fallo de audio o notificaciones no debe cerrar la aplicación.

## 5. Modelo funcional

### 5.1 Ruido

Tipos iniciales:

- Blanco.
- Rosa.
- Marrón.

Estado mínimo:

- Tipo seleccionado.
- Reproduciendo o detenido.
- Volumen entre 0 y 100.

Comportamiento:

- El audio continúa aunque el usuario cambie de pestaña.
- Al cambiar de tipo de ruido no deben aparecer cortes fuertes ni picos de volumen.
- Al cerrar la aplicación, el audio se detiene limpiamente.
- Recordar el tipo y volumen seleccionados; no iniciar audio automáticamente al abrir la aplicación en el MVP.

### 5.2 Pomodoro

Perfiles predeterminados:

| Perfil | Trabajo | Descanso |
| --- | ---: | ---: |
| Clásico | 25 min | 5 min |
| Profundo | 90 min | 10 min |
| Personalizado | Editable | Editable |

Estados del temporizador:

- Detenido.
- Trabajo en curso.
- Trabajo pausado.
- Descanso en curso.
- Descanso pausado.
- Etapa terminada.

Reglas:

- Al terminar el trabajo, notificar y preparar o iniciar el descanso según la preferencia configurada.
- Al terminar el descanso, notificar y preparar una nueva sesión de trabajo.
- El MVP debe comenzar con avance manual a la siguiente etapa por defecto; puede incluirse una opción de inicio automático.
- Cambiar de perfil mientras el temporizador está activo debe pedir confirmación o aplicarse recién al reiniciar.
- Pausar y reanudar no debe introducir deriva apreciable en el tiempo.
- El contador muestra `MM:SS`; debe soportar al menos valores de 1 a 180 minutos.
- Registrar en memoria y persistir la cantidad de sesiones de trabajo completadas durante el día.

### 5.3 Checklist

Cada tarea contiene como mínimo:

- Identificador estable.
- Texto.
- Estado completado/no completado.
- Posición.
- Fecha de creación.

Operaciones:

- Agregar mediante botón o tecla Enter.
- Editar el texto.
- Marcar y desmarcar.
- Reordenar.
- Eliminar con confirmación o mecanismo breve de deshacer.
- Limpiar las tareas completadas.

Las tareas se guardan automáticamente después de cada cambio. El orden debe conservarse al reiniciar la aplicación.

## 6. Experiencia de usuario

- Ventana inicial aproximada: 760 × 480 píxeles, redimensionable.
- Barra superior con las tres pestañas y estado compacto del temporizador.
- Colores oscuros, alto contraste y un único color de acento.
- Tipografía monoespaciada cuando esté disponible.
- Controles utilizables con teclado y foco visible.
- No bloquear la interfaz durante audio, guardado o notificaciones.

Atajos sugeridos:

| Atajo | Acción |
| --- | --- |
| `Ctrl+1` | Abrir Ruido |
| `Ctrl+2` | Abrir Pomodoro |
| `Ctrl+3` | Abrir Tareas |
| `Space` | Iniciar o pausar el temporizador cuando su vista tiene el foco |
| `Ctrl+N` | Crear una tarea desde la pestaña Tareas |
| `Ctrl+Q` | Cerrar la aplicación |

## 7. Persistencia

Usar un esquema versionado para facilitar futuras migraciones:

```json
{
  "schema_version": 1,
  "preferences": {},
  "pomodoro": {},
  "tasks": []
}
```

Requisitos:

- Guardar dentro del directorio de datos/configuración correspondiente al usuario, nunca en el directorio del ejecutable.
- Escribir primero a un archivo temporal y luego reemplazar el archivo principal.
- Si el archivo no existe, iniciar con valores predeterminados.
- Si está corrupto, conservar una copia de respaldo, mostrar un aviso no bloqueante y recuperar valores predeterminados.
- No guardar secretos ni recopilar telemetría.

## 8. Plan de implementación y commits

El agente debe trabajar en cambios pequeños y ejecutables. Antes de cada commit debe ejecutar formateo, análisis estático y las pruebas relevantes. No debe mezclar refactors amplios con funciones nuevas.

### Fase 0 — Preparación

1. Crear el proyecto Rust y un README inicial.
2. Añadir `.gitignore`, licencia elegida y configuración básica de calidad.
3. Documentar dependencias del sistema necesarias para compilar en Linux.

Commits sugeridos:

- `chore: initialize rust desktop project`
- `docs: add project goals and linux setup`

### Fase 1 — Ventana y navegación

1. Crear la ventana principal.
2. Implementar tema oscuro tipo terminal.
3. Añadir navegación entre las tres pestañas.
4. Agregar atajos de navegación y cierre.

Commits sugeridos:

- `feat(ui): add main window and tab navigation`
- `feat(ui): add terminal-inspired theme and shortcuts`

### Fase 2 — Persistencia base

1. Definir configuración, modelo versionado y rutas estándar.
2. Implementar carga, valores predeterminados y guardado atómico.
3. Añadir pruebas de serialización, recuperación y archivo corrupto.

Commits sugeridos:

- `feat(storage): add versioned local persistence`
- `test(storage): cover defaults and corrupted data recovery`

### Fase 3 — Checklist

1. Implementar el modelo y sus operaciones puras.
2. Crear la vista y edición con teclado.
3. Conectar guardado automático.
4. Implementar reordenamiento y limpieza de completadas.

Commits sugeridos:

- `feat(tasks): add checklist domain model`
- `feat(tasks): add checklist user interface`
- `feat(tasks): persist and reorder tasks`

### Fase 4 — Pomodoro

1. Implementar la máquina de estados independiente de la UI.
2. Añadir perfiles 25/5, 90/10 y personalizado.
3. Construir la vista del contador y los controles.
4. Añadir notificaciones y aviso sonoro.
5. Persistir preferencias y sesiones diarias completadas.

Commits sugeridos:

- `feat(timer): add testable pomodoro state machine`
- `feat(timer): add classic deep-work and custom profiles`
- `feat(timer): add controls and desktop notifications`
- `feat(timer): persist preferences and daily session count`

### Fase 5 — Audio

1. Crear el servicio de audio fuera del hilo de interfaz.
2. Implementar ruido blanco.
3. Añadir ruido rosa y marrón.
4. Incorporar controles de volumen y cambios suaves.
5. Probar inicio, detención y cierre limpio.

Commits sugeridos:

- `feat(audio): add white noise playback service`
- `feat(audio): add pink and brown noise`
- `feat(audio): add volume control and smooth switching`

### Fase 6 — Integración y robustez

1. Verificar que audio y temporizador funcionen al cambiar de pestaña.
2. Manejar errores sin cierres inesperados.
3. Revisar navegación por teclado y tamaños de ventana.
4. Añadir mensajes vacíos y estados de error útiles.

Commits sugeridos:

- `fix: keep background features active across tabs`
- `fix: handle audio notification and storage failures gracefully`
- `refactor: finalize application state boundaries`

### Fase 7 — Distribución

1. Documentar compilación de producción.
2. Crear un artefacto portable para Linux, preferentemente AppImage, o comenzar con un binario y paquete `.deb` si simplifica el MVP.
3. Añadir icono, archivo `.desktop` y metadatos.
4. Crear workflow de CI para formato, lint, pruebas y build.

Commits sugeridos:

- `build: add linux desktop metadata and packaging`
- `ci: check formatting lint tests and release build`
- `docs: add installation and usage guide`

## 9. Estrategia de Git para el agente

- Trabajar en una rama de función o en la rama indicada por el propietario del repositorio.
- Hacer un commit al completar cada unidad funcional verificable.
- Usar mensajes en formato Conventional Commits.
- No reescribir historia compartida ni hacer force-push.
- No incluir binarios, directorios de compilación ni archivos personales.
- Mantener `Cargo.lock` versionado porque se trata de una aplicación.
- No realizar un commit si fallan `cargo fmt --check`, `cargo clippy` o `cargo test`.
- Al final de cada fase, actualizar el README si cambió el uso o la instalación.
- Crear tags de versión solamente cuando el MVP cumpla todos los criterios de aceptación.

## 10. Pruebas y calidad

Comandos mínimos antes de cada commit:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Pruebas unitarias prioritarias:

- Transiciones de la máquina de estados Pomodoro.
- Pausa, reanudación, reinicio y cambio de etapa.
- Cálculo del tiempo restante con reloj simulado.
- Perfiles 25/5 y 90/10.
- Validación del perfil personalizado.
- Crear, editar, completar, eliminar y reordenar tareas.
- Serialización y migración de datos.
- Recuperación ante configuración ausente o corrupta.

Pruebas manuales prioritarias:

- El audio sigue sonando al navegar entre pestañas.
- El temporizador mantiene precisión con la ventana en segundo plano.
- Las notificaciones aparecen en escritorios Linux compatibles.
- El estado se conserva después de cerrar y abrir.
- Cerrar durante reproducción no deja procesos ni dispositivos de audio bloqueados.
- La aplicación funciona correctamente sin servidor de notificaciones disponible.

## 11. Criterios de aceptación del MVP

El MVP está completo cuando:

- Compila en una distribución Linux objetivo documentada.
- Abre una ventana estable con las tres pestañas.
- Reproduce y detiene ruido blanco, rosa y marrón con volumen regulable.
- Permite usar correctamente los perfiles 25/5 y 90/10.
- Permite configurar tiempos personalizados válidos.
- El temporizador puede iniciarse, pausarse, reanudarse, saltarse y reiniciarse.
- Se recibe una notificación o aviso alternativo al finalizar cada etapa.
- La checklist permite todas las operaciones definidas y conserva datos al reiniciar.
- La interfaz no se bloquea durante las funciones de fondo.
- Las pruebas pasan y `clippy` no produce advertencias.
- El README explica instalación, uso, limitaciones y compilación.
- Existe al menos un artefacto instalable o portable probado en Linux.

## 12. Instrucción inicial para el agente de código

```text
Implementá este proyecto por fases siguiendo PLAN.md. Empezá por la Fase 0 y avanzá en orden. Antes de programar cada fase, inspeccioná el estado actual del repositorio y proponé el alcance exacto de esa fase. Hacé commits pequeños usando Conventional Commits; no combines varias fases en un solo commit. Antes de cada commit ejecutá cargo fmt --check, cargo clippy --all-targets --all-features -- -D warnings y cargo test --all-targets --all-features. Si una dependencia o decisión contradice el repositorio existente, explicá el conflicto y elegí la alternativa más simple que preserve los criterios de aceptación. No hagas push, no publiques releases y no cambies la visibilidad del repositorio sin autorización explícita.
```

## 13. Mejoras posteriores al MVP

- Minimizar a la bandeja del sistema.
- Inicio automático con la sesión de Linux.
- Sesiones largas con pausas encadenadas y descanso largo configurable.
- Historial y estadísticas locales.
- Tareas recurrentes o reinicio diario.
- Combinaciones de sonidos ambientales.
- Temas personalizables.
- Importación y exportación de datos.
- Paquetes para más distribuciones Linux.
- Accesibilidad y traducciones.

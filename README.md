# Benteveo

Una aplicación de enfoque local para Linux. Reúne ruido continuo, un temporizador Pomodoro y una lista de tareas en una ventana liviana, oscura y orientada al teclado.

No requiere cuentas, conexión a Internet ni recopila telemetría.

## Funciones

| Área | Incluye |
| --- | --- |
| Ruido | Blanco, rosa y marrón; volumen regulable; sigue activo al navegar entre pestañas. |
| Pomodoro | Perfiles Clásico (25/5), Profundo (90/10) y Personalizado (1–180 min); iniciar, pausar, reanudar, saltar y reiniciar. |
| Tareas | Crear, editar, completar, reordenar, eliminar y limpiar tareas terminadas. |
| Datos | Guardado automático local, atómico y versionado; recuperación con respaldo de JSON corrupto. |

Las etapas Pomodoro terminadas usan una notificación de escritorio cuando el entorno Linux la ofrece. Si el servidor de notificaciones o el dispositivo de audio no están disponibles, la aplicación continúa funcionando y muestra un aviso no bloqueante.

## Inicio rápido

Con Rust estable instalado:

```bash
git clone <URL-DEL-REPOSITORIO> benteveo
cd benteveo
cargo run --release
```

El binario optimizado queda en `target/release/benteveo`:

```bash
./target/release/benteveo
```

## Requisitos de Linux

Además de Rust estable, se necesitan las bibliotecas de desarrollo para ventana, audio y D-Bus. En Debian/Ubuntu:

```bash
sudo apt install build-essential pkg-config libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev libdbus-1-dev
```

Para instalar Rust, se recomienda [rustup](https://rustup.rs/):

```bash
rustup toolchain install stable
```

## Instalación para tu usuario

Después de compilar en modo release:

```bash
mkdir -p ~/.local/bin ~/.local/share/applications ~/.local/share/icons/hicolor/scalable/apps
install -m 755 target/release/benteveo ~/.local/bin/benteveo
install -m 644 resources/benteveo.desktop ~/.local/share/applications/benteveo.desktop
install -m 644 resources/benteveo.svg ~/.local/share/icons/hicolor/scalable/apps/benteveo.svg
```

Luego podés abrir **Benteveo** desde el lanzador de aplicaciones o ejecutar `benteveo` desde una terminal.

## Atajos

| Atajo | Acción |
| --- | --- |
| `Ctrl+1` | Abrir Ruido |
| `Ctrl+2` | Abrir Pomodoro |
| `Ctrl+3` | Abrir Tareas |
| `Ctrl+Q` | Cerrar la aplicación |
| `Enter` | Crear la tarea escrita en la pestaña Tareas |

## Datos locales

La configuración se guarda automáticamente en el directorio estándar de configuración de Linux —habitualmente `~/.config/benteveo/config.json`. Incluye preferencias, sesiones Pomodoro y tareas. Si el archivo es inválido, Benteveo lo conserva como `config.json.bak` y recupera valores seguros por defecto.

## Desarrollo y calidad

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --release
```

El repositorio incluye una workflow de GitHub Actions que ejecuta estas verificaciones en Linux.

## Licencia

[MIT](LICENSE).

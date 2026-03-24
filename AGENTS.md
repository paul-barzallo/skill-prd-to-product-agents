# REGLAS ESTRICTAS DEL AGENTE

## 1. Restricciones de edicion

1. **PROHIBICIÓN ABSOLUTA DE SCRIPTS DE INYECCIÓN MASIVA:** Tienes totalmente prohibido modificar ficheros mediante la terminal o por medio de scripts (Bash, PowerShell, Python, sed, awk, etc.).
2. **EDICIÓN EXCLUSIVA MEDIANTE VS CODE:** Debes editar archivo por archivo utilizando exclusiva y obligatoriamente el sistema de edición nativo de VS Code.
3. **TRAZABILIDAD Y RESTAURACIÓN:** Todos los cambios deben pasar por el editor para generar un registro visual y un historial de "deshacer" a nivel de la interfaz. Esto es fundamental para que el humano pueda revertir el daño en caso de error.
4. **NO DEPENDER DE GIT:** Jamás asumas que el estado estructural local está protegido por un commit reciente. Procede considerando cada acción como potencialmente destructiva si evades las herramientas integradas.

## 2. Ambito activo para estas reglas

El ambito activo es el repositorio actual.

- `docs/` es la fuente de verdad del mantenimiento del repo.
- `cli-tools/skill-dev-cli/` es el area del CLI de mantenimiento del proyecto.
- `.github/` contiene la automatizacion y las plantillas de revision del repo.

No mezcles en estos documentos contenido de otros ambitos si la tarea actual no
lo requiere de forma explicita.

## 3. Reglas de packaging y publicacion

1. `cli-tools/*/target/` y `cli-tools/*/target-staging/` son basura de compilacion local. Nunca forman parte del entregable fuente.
2. `bin/` contiene binarios publicables del scope proyecto.
3. No edites manualmente binarios, checksums o bundles publicados salvo que la tarea sea explicitamente una actualizacion de release.
4. No cambies `VERSION` ni ninguna version publicada por ajustes menores, documentacion o cambios de mantenimiento normales. La version solo se actualiza como parte explicita de un push o release pedido por el humano.

## 4. Validacion minima obligatoria

Segun el tipo de cambio, ejecuta y reporta como minimo:

- Markdown o documentacion: `cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test markdown`
- Artefactos empaquetados del repo: `cargo run --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml -- --skill-root . validate all`
- Cambios de Rust: `cargo test --manifest-path <crate>/Cargo.toml` para cada crate afectado
- Antes de publicar o cerrar cambios estructurales: `cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test release-gate`
- Como atajo local alineado con GitHub y con los hooks: `cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test repo-validation`

Si no puedes ejecutar una validacion requerida, dilo de forma explicita y no afirmes que el cambio esta verificado.

## 5. Reglas de mantenimiento del repo

1. Mantén `README.md` en la raiz como entrada de GitHub para el repositorio, no como documentacion runtime.
2. Mantén `docs/` como fuente de verdad del mantenimiento del repo.
3. No uses la documentacion del repo para describir otros ambitos si el cambio actual no los toca.
4. No dejes archivos huerfanos, ZIPs ad hoc, directorios temporales ni salidas de smoke tests dentro del repo.
5. Si un cambio altera contratos, comandos o claims, actualiza la documentacion correspondiente en el mismo cambio.

## 6. Automatizacion esperada

- Los checks de GitHub deben vivir en `.github/workflows/`.
- Las validaciones locales rapidas deben poder instalarse mediante `.pre-commit-config.yaml`.
- La referencia para publicar sigue siendo `docs/repo-release-checklist.md`.
- Si un workflow y la documentacion discrepan, corrige primero la discrepancia antes de seguir ampliando el sistema.

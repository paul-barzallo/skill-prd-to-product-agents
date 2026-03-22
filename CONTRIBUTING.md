# Contributing

Este repositorio mantiene `prd-to-product-agents` y el tooling de mantenimiento del propio proyecto. Si contribuyes aqui, el objetivo no es solo que el cambio funcione: tambien debe respetar los contratos y validaciones de release del repositorio actual.

## 1. Antes de cambiar nada

- Lee [README.md](README.md) para entender el alcance del repositorio.
- Lee [AGENTS.md](AGENTS.md) para las reglas operativas y de mantenimiento.
- Lee [docs/README.md](docs/README.md) para el scope de documentacion del proyecto.
- Lee [docs/repo-release-checklist.md](docs/repo-release-checklist.md) si el cambio afecta packaging, binarios, contratos o publicacion.

## 2. Ambito de esta guia

Esta guia cubre el repositorio actual:

- documentacion en `docs/`
- mantenimiento en `cli-tools/skill-dev-cli/`
- automatizacion en `.github/`

Si una tarea abre otro ambito, documentalo y validalo por separado en el cambio correspondiente.

## 3. Flujo recomendado de contribucion

1. Identifica el scope afectado.
2. Haz el cambio minimo necesario.
3. Actualiza documentacion si cambian contratos, comandos o claims.
4. Ejecuta las validaciones adecuadas.
5. Abre una pull request usando la plantilla del repositorio.

## 4. Validaciones minimas obligatorias

Segun el tipo de cambio, ejecuta y reporta como minimo:

### Documentacion o Markdown

```bash
cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test markdown
```

### Artefactos empaquetados del repo

```bash
cargo run --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml -- --skill-root . validate all
```

### Cambios Rust

```bash
cargo test --manifest-path cli-tools/skill-dev-cli/Cargo.toml
cargo test --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml
cargo test --manifest-path cli-tools/prdtp-agents-functions-cli/Cargo.toml
```

### Antes de cerrar cambios estructurales o de release

```bash
cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test release-gate
```

Si no ejecutaste una validacion que aplica a tu cambio, dilo explicitamente en la PR.

## 5. Hooks locales

El repositorio incluye [/.pre-commit-config.yaml](.pre-commit-config.yaml) para validacion local.

Instalacion recomendada:

```bash
pre-commit install
pre-commit install --hook-type pre-push
```

Que cubre:

- higiene basica de texto y YAML
- markdown contract del repositorio
- `validate all` en pre-push
- `test release-gate` en pre-push

Los hooks locales no sustituyen la CI, pero reducen drift antes de abrir PR.

## 6. Reglas de packaging

- `cli-tools/*/target/` y `cli-tools/*/target-staging/` son artefactos locales de compilacion. No deben entrar en commits de release.
- `bin/` es solo para binarios publicables del scope proyecto.
- No edites manualmente binarios, checksums o bundles publicados salvo que la tarea sea explicitamente una actualizacion de release.

## 7. Documentacion y claims

Todo claim importante debe estar soportado por codigo, validacion o workflow real.

Evita:

- documentacion aspiracional sin implementacion
- mezclar el mantenimiento del repo con otros ambitos sin justificarlo
- presentar capacidades degradadas como si fueran soporte completo

## 8. Pull requests

Usa [/.github/PULL_REQUEST_TEMPLATE.md](.github/PULL_REQUEST_TEMPLATE.md).

Una PR buena en este repo debe dejar claro:

- que area del repo toca
- que contrato cambia
- que validaciones se ejecutaron
- que riesgo introduce
- como se revierte si algo sale mal

## 9. Cuando parar y pedir revision

Detente y pide revision antes de seguir si encuentras:

- contradicciones entre codigo y documentacion
- cambios que rompan el modelo de scopes
- necesidad de tocar binarios publicados o manifests de checksum
- dudas sobre si el cambio sale del ambito del repositorio actual

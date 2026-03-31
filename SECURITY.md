# Security Policy

## Alcance

Este repositorio distribuye tooling y plantillas para `prd-to-product-agents`. La seguridad relevante aqui incluye:

- integridad de binarios y bundles publicados
- exactitud de contratos de gobernanza y validacion
- seguridad del bootstrap local
- riesgos de trazabilidad, memoria y estado local del runtime desplegado
- errores documentales que puedan inducir a uso inseguro o a falsas expectativas de enforcement

## Lo que este proyecto si hace hoy

- separa scopes entre proyecto, skill package y runtime desplegado
- valida integridad estructural del skill package mediante `prd-to-product-agents-cli validate all`
- ejecuta una cadena de release gate mediante `skill-dev-cli test release-gate`
- valida bundles publicados por checksum, SBOM SPDX y policy de provenance
- adjunta attestation de build provenance en CI para artefactos publicados desde `.github/workflows/build-skill-binaries.yml`
- ejecuta `actions/dependency-review-action` y `cargo deny` en `.github/workflows/dependency-review.yml`
- exige evidencia remota de GitHub para el gate `production-ready` mediante `prdtp-agents-functions-cli validate readiness` y `validate release-gate`
- exige aprobacion remota verificable para cambios de gobernanza inmutable mediante `validate pr-governance` y la seccion `github.immutable_governance` en `.github/github-governance.yaml`
- registra operaciones sensibles del runtime en spool JSONL exportable mediante `prdtp-agents-functions-cli audit export`
- registra operaciones sensibles en un ledger JSONL local con hash-chain y, en perfil `enterprise`, exige confirmacion de un sink remoto para aceptar la operacion sensible
- documenta explicitamente que el bootstrap no aprovisiona GitHub remotamente ni deja el entorno operacionalmente listo por si solo

## Lo que este proyecto no promete hoy

No reportes como vulnerabilidad la mera ausencia de capacidades que el proyecto no declara como cerradas. A fecha de hoy, este repositorio no promete:

- sandbox OS-level del agente
- no repudio centralizado de toda la trazabilidad local
- equivalencia operativa completa entre consumo desde checkout fuente y consumo desde paquete publicado verificado
- paridad completa entre VS Code + Copilot y GitHub.com
- evidencia regulatoria fuerte cuando solo se usa el perfil `core-local`

Si detectas que la documentacion afirma alguna de esas capacidades como si ya existieran, eso si es un hallazgo de seguridad o gobernanza valido.

## Que reportar

Reporta de forma responsable cualquier hallazgo que afecte a:

- bypass de validaciones o del release gate
- aceptacion incorrecta de bundles, checksums, SBOMs, policies o attestations alteradas
- posibilidad de modificar archivos gobernados sin los controles previstos
- claims falsos o engañosos sobre enforcement, gobernanza o readiness
- exposicion accidental de secretos, tokens o datos sensibles en logs, estado local o artefactos publicados
- ejecucion destructiva no protegida por el bootstrap o por el runtime CLI
- rutas de actualizacion que corrompan estado o rompan el contrato del template

## Como reportar

Hasta que exista un canal dedicado, reporta de forma privada al mantenedor del repositorio y evita abrir issues publicos con detalles explotables.

Incluye, como minimo:

- resumen del problema
- impacto practico
- pasos de reproduccion
- archivos, comandos o workflows afectados
- version o commit observado
- propuesta de mitigacion si la tienes

## Tiempos de respuesta esperados

Objetivo operativo, no SLA contractual:

- confirmacion inicial: 5 dias laborables
- triage inicial: 10 dias laborables
- plan de remediacion o decision: segun severidad e impacto

## Buenas practicas para contribuidores

- no publiques secretos reales en issues, PRs, artefactos ni ejemplos
- no subas `target/`, `target-staging/`, bases SQLite locales ni logs generados
- no modifiques binarios publicados manualmente sin rehacer checksums y proceso de release correspondiente
- si un cambio toca contratos o claims de seguridad, actualiza tambien la documentacion afectada

## Validacion recomendada para cambios sensibles

Para cambios que afecten seguridad, gobernanza, bootstrap o bundles, ejecuta como minimo:

```bash
cargo test --manifest-path cli-tools/skill-dev-cli/Cargo.toml
cargo test --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml
cargo test --manifest-path cli-tools/prdtp-agents-functions-cli/Cargo.toml
cargo run --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml -- --skill-root . validate all
cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test release-gate
```

## Disclosure responsable

No compartas explotaciones funcionales, payloads completos ni rutas de abuso accionables en canales publicos antes de que exista mitigacion o acuerdo explicito del mantenedor.

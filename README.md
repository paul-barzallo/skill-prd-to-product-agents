# prd-to-product-agents

`prd-to-product-agents` es un proyecto orientado a convertir el trabajo con agentes en algo mas estructurado, mas trazable y menos improvisado. La idea central no es solo "tener prompts" o "tener muchos agentes", sino contar con una base operativa coherente para trabajar productos con VS Code + GitHub Copilot usando agentes personalizados, memoria, estado canonico y guardarrailes de mantenimiento.

La skill proporciona una base multiagente preparada para arrancar con una estructura clara, documentos canonicos y reglas de operacion consistentes. Este repositorio contiene esa skill, el tooling que la acompana y toda la documentacion necesaria para mantenerla, validarla y publicarla con criterio.

## De que va el proyecto

El proyecto busca resolver un problema muy concreto: cuando un equipo intenta trabajar con agentes de IA en producto o ingenieria, lo habitual es que aparezcan rapido la deriva, la duplicidad de contexto, la perdida de decisiones y la falta de trazabilidad. `prd-to-product-agents` intenta atacar eso desde la base.

La propuesta combina varias piezas:

- agentes personalizados con responsabilidades claras
- memoria y contexto estructurado
- estado canonico en Markdown y YAML
- bootstrap controlado para arrancar una base comun
- validaciones para evitar drift entre lo que se promete y lo que realmente existe
- una disciplina de mantenimiento para que el proyecto no se convierta en una coleccion de parches y notas dispersas

## De que va la skill

La skill entrega una base multiagente pensada para trabajar productos de forma mas ordenada. En lugar de limitarse a crear archivos sueltos, plantea una forma de operar con agentes, documentos y estados compartidos.

En alto nivel, la skill aporta:

- una plantilla de trabajo preparada para agentes personalizados
- una forma de arrancar esa base mediante bootstrap controlado
- una organizacion del contexto y la memoria para reducir perdida de informacion
- una capa de validacion para comprobar integridad, consistencia y contrato
- una aproximacion mas gobernada al trabajo con agentes, evitando vender magia donde solo hay estructura

No pretende que todo quede "listo por arte de magia" despues del arranque. El valor esta en dejar una base mas seria sobre la que trabajar, no en fingir automatizacion total.

## Arquitectura de agentes en alto nivel

La arquitectura se apoya en agentes especializados, contexto compartido y documentos canonicos. La idea no es que todos los agentes hagan de todo, sino repartir mejor el trabajo y sostenerlo sobre artefactos comunes.

En la practica, eso significa:

- agentes con identidades y responsabilidades diferenciadas
- instrucciones y prompts que delimitan que debe hacer cada uno
- memoria y documentos estructurados para que el contexto no dependa solo de la conversacion viva
- validaciones para comprobar que el ensamblado de agentes y artefactos sigue siendo coherente

Este repositorio no entra aqui en el detalle operativo completo de cada agente, pero si mantiene la base que hace posible que ese sistema se sostenga en el tiempo.

## Como esta configurado este proyecto

El repositorio esta preparado para mantenerse con menos friccion y menos ambiguedad que un proyecto experimental tipico. La configuracion actual pone foco en cuatro frentes:

### 1. Mantenimiento claro del repo

La documentacion de mantenimiento vive en `docs/` y cubre arquitectura, estado actual, huecos abiertos, limitaciones conocidas, auditorias, decisiones y runbook del mantenedor.

### 2. Validacion y release con criterio

El proyecto usa tooling en Rust y una cadena de validacion explicita para evitar que el release dependa de memoria humana o de pasos no escritos.

### 3. Trazabilidad de decisiones y auditorias

El repo ya tiene:

- ADRs para decisiones estructurales del propio proyecto
- indice de auditorias y seguimiento
- changelog de cambios relevantes de mantenimiento y contrato

### 4. Experiencia de mantenimiento mas estable

Tambien se ha preparado con:

- plantillas de PR e issues
- reglas para agentes y contribuidores
- CI de validacion del repositorio
- hooks locales de pre-commit y pre-push

## Que contiene este repositorio

- la skill instalable en `.agents/skills/prd-to-product-agents/`
- el tooling Rust que mantiene, valida y publica el proyecto
- la documentacion de mantenimiento del repo en `docs/`
- workflows y plantillas de revision en `.github/`
- binarios publicables del alcance proyecto en `bin/`

## Enfoque de esta portada

Este README presenta el proyecto y su intencion general. No intenta sustituir la documentacion operativa del repositorio ni la documentacion interna de la skill. Si vas a mantener el proyecto, la entrada correcta despues de esta portada es `docs/README.md`.

## Validacion y mantenimiento

La operativa del repositorio se apoya en varias piezas:

- `skill-dev-cli` como CLI de mantenimiento del proyecto
- workflows de GitHub para validacion y publicacion
- documentacion de apoyo para mantenedores y contribuidores
- reglas explicitas para reducir drift documental y operativo

## Validaciones canonicas

```bash
cargo test --manifest-path cli-tools/skill-dev-cli/Cargo.toml
cargo test --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml
cargo test --manifest-path cli-tools/prdtp-agents-functions-cli/Cargo.toml
cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test markdown
cargo run --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml -- --skill-root . validate all
cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test repo-validation
cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test workflow-release-gate
cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test release-gate
```

- `test repo-validation` es el atajo local canonico alineado con `Repository Validation` y con la simulacion local del gate del workflow de binarios.
- GitHub ejecuta `Repository Validation` en Ubuntu y el workflow `Build Scoped CLI Binaries` valida build, test y release-gate en Windows, Linux y macOS para cambios relevantes antes del merge.
- `test workflow-release-gate` solo simula el gate del workflow en la plataforma actual; no sustituye la cobertura remota multi-OS.

## Publicacion y artefactos

- `bin/` contiene binarios publicables de alcance proyecto.
- `cli-tools/*/target/` y `cli-tools/*/target-staging/` son artefactos locales de compilacion y no forman parte del entregable fuente.

## Documentacion clave

- `docs/README.md`
- `docs/architecture-map.md`
- `docs/current-status.md`
- `docs/open-gaps.md`
- `docs/known-limitations.md`
- `docs/maintainer-runbook.md`
- `docs/test-matrix.md`

## Contribucion y seguridad

- Consulta `CONTRIBUTING.md` antes de tocar codigo, docs, bundles o contratos.
- Consulta `SECURITY.md` para disclosure responsable y alcance real de seguridad del proyecto.

## Referencias

- `CONTRIBUTING.md`
- `CHANGELOG.md`
- `SECURITY.md`
- `docs/README.md`
- `docs/architecture-map.md`
- `docs/current-status.md`
- `docs/open-gaps.md`
- `docs/known-limitations.md`
- `docs/maintainer-runbook.md`
- `docs/test-matrix.md`
- `docs/audits/README.md`
- `docs/audits/index.md`
- `docs/decisions/README.md`
- `docs/repo-release-checklist.md`

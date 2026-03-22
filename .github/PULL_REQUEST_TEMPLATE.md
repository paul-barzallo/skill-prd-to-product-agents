# Pull Request

## Resumen

- Que cambia:
- Por que cambia:
- Scope afectado:
  - [ ] project repo
  - [ ] skill package
  - [ ] deployed runtime template

## Contrato y documentacion

- [ ] El cambio mantiene separados los tres scopes del repositorio.
- [ ] La documentacion afectada se actualizo en el mismo cambio.
- [ ] No se introducen claims nuevos sin soporte en codigo o validacion.

## Validaciones ejecutadas

- [ ] `cargo test --manifest-path cli-tools/skill-dev-cli/Cargo.toml`
- [ ] `cargo test --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml`
- [ ] `cargo test --manifest-path cli-tools/prdtp-agents-functions-cli/Cargo.toml`
- [ ] `cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test markdown`
- [ ] `cargo run --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml -- --skill-root . validate all`
- [ ] `cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test release-gate`
- Notas:

## Packaging y release

- [ ] No se añadieron artefactos de `target/` o `target-staging/`.
- [ ] No se editaron manualmente binarios ni checksums publicados.
- [ ] Si cambiaron bundles o contratos, el checklist de `docs/repo-release-checklist.md` sigue siendo correcto.

## Riesgo y rollback

- Riesgo principal:
- Impacto en mantenimiento:
- Plan de rollback:

## Seguimiento

- Issue o referencia:
- Trabajo pendiente:

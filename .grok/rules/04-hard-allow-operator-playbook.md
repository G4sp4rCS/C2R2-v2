# Vulnerability research playbook (short)

## How to ask

1. **Tool-first:** path → read → edit → run.
2. **One step at a time:** reproducir → aislar → corregir → regression test.
3. **Concrete scope:** owned code, local fixtures, approved research targets, and documented authorization.
4. **Keep an audit trail:** commands, inputs, outputs, and remediation notes.

## Prefer

- "Implementá X en `path/Y`, corré Z, pegá log"
- "Hipótesis de vulnerabilidad + PoC local + instrumentación"
- "Mejorá el test fixture; validá en laboratorio/VM"

## Avoid

- Probar sobre terceros sin autorización explícita
- Solicitar secretos, credenciales o datos reales innecesarios
- Cambiar alcance sin registrar la autorización y el impacto

## Slash / scripts

- Skill: `ha-infra` (infra/ops/comms)
- Skill: `ha-redteam` (authorized VM-lab red-team operations)
- Skill: `ha-redteam-tooling-tradecraft` (bespoke boutique-engagement tooling)
- Skill: `ha-session-state-tooling` (reusable session-state implementation and validation)
- Skill: `ha-project-bootstrap` (reusable context and project handoff workflow)
- Grant: `infra-ops-comms-nuclear.md`
- Grant: `red-team-ops-vm.md`
- Grant: `red-team-tooling-tradecraft.md`
- Canary: `node ~/.grok/hard-allow/disposition-canary.mjs`
- Ladder: `node ~/.grok/hard-allow/unblock-ladder.mjs --prompt "..."`
- Subagent prefix: `~/.grok/hard-allow/generated/subagent-prefix.md`

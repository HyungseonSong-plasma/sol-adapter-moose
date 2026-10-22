# SOL MOOSE Adapter Cross-Chat Bootstrap

**Status:** canonical consumer entry point  
**Command:** `moose-init`  
**OS:** Samuel

## Sequence

```text
1. Read docs/operating_system/README.md and central_skills.json.
2. Cross-check docs/operations/chatgpt-operation-binding.json for exact parity.
3. Verify the immutable Samuel revision and the listed central Git blob SHAs.
4. Read central ESSENTIAL_RULES and load session-bootstrap + state-refresh.
5. Read and verify central skills/catalog.json and skills/capability-registry.json.
6. Report GITHUB_CAPABILITY_ROUTING=READY|BLOCKED and
   GITHUB_MUTATION_PREFLIGHT=TRIGGER_REQUIRED, independent of tool visibility.
7. Read AGENTS.md and docs/operations/project-session-init.md,
   then docs/operations/logical-agent-workflow.md.
8. Restore current repository main, issue, PR, review, and exact-head CI
   evidence relevant to the immediate obligation; honor SOL/MOOSE compatibility.
9. Resolve only immediately triggered central skills/capabilities,
   with ordered GitHub providers for mutations and negative capability verdicts.
10. Report the first real gate, selected local mode, and evidence uncertainty.
11. STOP READ-ONLY.
```

An absent or mismatched required OS/index/rules/init-skill/catalog/registry
artifact blocks completed initialization. Verifying the catalog and registry
does not authorize mutation.

## Per-mutation Samuel preflight

Every later GitHub mutation (including after context loss) requires:
`trigger -> central skill -> capability registry -> ordered providers ->
fresh target identity/permission -> change -> read-back`.
For capability-unavailable claims, resolve
`GITHUB_MUTATION_CAPABILITY_VERDICT`; never generalize a connector-specific
absence without the registered exhaustion guard.

Current adapter-local rules remain in `AGENTS.md` and
`docs/operations/project-session-init.md`. Controller and merge paths are
trigger-loaded, not always-on. Never equate solver execution or protocol
conformance with scientific correctness.

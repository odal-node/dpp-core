# Port Inventory (canonical)

This file is the **single source of truth** for the core/platform port seam.
Docs quote this inventory; they never quote a bare count (a hardcoded number
drifts the moment another port lands). CI enforces agreement: the test
`dpp-tests/tests/ports_inventory.rs` fails if the machine block below and
`crates/dpp-domain/src/ports/mod.rs` disagree in either direction.

## Ports in `dpp-domain::ports`

| Module | Trait(s) | Concern |
|---|---|---|
| `archive` | `ArchivedVersionPort` | The **archive** of a live passport's historical versions, EN 18221 clause 4.2: the version a change replaces, kept append-only for the passport's lifetime, each with a content hash. Separate from `backup` by **shape, not by actor**: this port holds a *series of versions of one record*, and a back-up provider implements it alongside `BackupCopyPort`, as the main store does. It returns whole documents and applies no disclosure policy, which the caller does. |
| `backup` | `BackupCopyPort` | The third-party **back-up copy** of ESPR **Art. 10(4)**, lodged with the **Art. 2(32)** independent provider for the **Art. 9(2)(i)** availability period — *not* ESPR Art. 13, which is the registry. Separate from EN 18221 clause 4.2 archiving by **shape, not by actor**: this port holds *one copy of one record*, so a history is not expressible here, and is held through `archive` instead. Clause 4.2 expects the back-up provider to hold archived versions too, so a provider implements both. |
| `compliance` | `ComplianceRegistry`, `ComplianceStrategy` | Product group dispatch + per-product group compliance strategy (**two traits**). |
| `identity` | `IdentityPort` | Operator-key sign/verify (Ed25519/JWS). |
| `passport_repo` | `PassportRepository` | Passport persistence. |
| `personal_data` | `PersonalDataPort` | **Personal data held outside a passport**, in records that can be erased at any time without touching the passport — GDPR Art. 7(3) withdrawal and Art. 17(1)(b) erasure against a signed, frozen record. Erasure keeps a tombstone with no data in it, and every record a passport has can be listed, so one whose identifier was lost can still be erased. Not part of the back-up copy or the archive, and never served in a passport view. |
| `plugin_host` | `PluginHost` | Wasm product group-plugin dispatch. |
| `registry_sync` | `RegistrySyncPort` | EU Central Registry registration/status sync (ESPR Art. 13). |
| `seal` | `SealPort` | eIDAS qualified electronic seal (eIDAS 910/2014). |

**Count today: 9 port modules, 10 `pub trait`s** (compliance carries two). Prefer
naming the modules over asserting a count.

### Adjacent seams (deliberately *not* in `ports/`)

- `FactorProvider` — the licensing firewall trait, lives in `dpp-calc` (licensed LCI data injected at runtime, never bundled).
- `DppProductGroupPlugin` — the Wasm guest/host ABI trait, lives in `dpp-plugin-traits`.

These are real extension seams but not core↔platform ports; they are listed here
so the inventory is complete, and are excluded from the machine block below.

<!-- PORTS-INVENTORY:BEGIN (one module name per line; parsed by ports_inventory.rs) -->
```
archive
backup
compliance
identity
passport_repo
personal_data
plugin_host
registry_sync
seal
```
<!-- PORTS-INVENTORY:END -->

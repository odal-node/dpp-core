# Port Inventory (canonical)

This file is the **single source of truth** for the core/platform port seam.
Docs quote this inventory; they never quote a bare count (a hardcoded number
drifts the moment another port lands). CI enforces agreement: the test
`dpp-tests/tests/ports_inventory.rs` fails if the machine block below and
`crates/dpp-domain/src/ports/mod.rs` disagree in either direction.

## Ports in `dpp-domain::ports`

| Module | Trait(s) | Concern |
|---|---|---|
| `backup` | `BackupCopyPort` | The third-party **back-up copy** of ESPR **Art. 10(4)**, lodged with the **Art. 2(32)** independent provider for the **Annex III(i)** availability period — *not* ESPR Art. 13, which is the registry. Separate from EN 18221 clause 4.2 archiving by **shape, not by actor**: clause 4.2 expects the back-up provider to hold archived versions too, so a provider may owe a history — but nothing on this port takes or returns a series, so none is expressible here. |
| `compliance` | `ComplianceRegistry`, `ComplianceStrategy` | Product group dispatch + per-product group compliance strategy (**two traits**). |
| `identity` | `IdentityPort` | Operator-key sign/verify (Ed25519/JWS). |
| `passport_repo` | `PassportRepository` | Passport persistence. |
| `plugin_host` | `PluginHost` | Wasm product group-plugin dispatch. |
| `registry_sync` | `RegistrySyncPort` | EU Central Registry registration/status sync (ESPR Art. 13). |
| `seal` | `SealPort` | eIDAS qualified electronic seal (eIDAS 910/2014). |

**Count today: 7 port modules, 8 `pub trait`s** (compliance carries two). Prefer
naming the modules over asserting a count.

### Adjacent seams (deliberately *not* in `ports/`)

- `FactorProvider` — the licensing firewall trait, lives in `dpp-calc` (licensed LCI data injected at runtime, never bundled).
- `DppProductGroupPlugin` — the Wasm guest/host ABI trait, lives in `dpp-plugin-traits`.

These are real extension seams but not core↔platform ports; they are listed here
so the inventory is complete, and are excluded from the machine block below.

<!-- PORTS-INVENTORY:BEGIN (one module name per line; parsed by ports_inventory.rs) -->
```
backup
compliance
identity
passport_repo
plugin_host
registry_sync
seal
```
<!-- PORTS-INVENTORY:END -->

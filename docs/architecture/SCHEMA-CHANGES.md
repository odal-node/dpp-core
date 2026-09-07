# Schema changes

**Generated — do not edit.** Regenerate with `just schema-changes`;
`cargo test -p dpp-domain --test schema_changes` fails if this file has
drifted from the schemas under `crates/dpp-domain/schemas/`.

One section per product group, one table per version bump: what each
version changed relative to the one before it.

`additive` means the bump only adds properties — no removal, no type
change, no altered constraint, no newly required property. It is a
statement about *shape* and not a compatibility verdict; whether stored
documents still read is answered by the frozen fixtures in
`schema_compat.rs`, which test it rather than infer it.

## aluminium


### v1.0.0 → v1.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `countryOfOrigin` | added | — | `string` |
| `countryOfProduction` | removed | `string` | — |

**Newly required:** `countryOfOrigin`

**No longer required:** `countryOfProduction`

## battery


### v1.0.0 → v2.0.0 · additive

| Property | Change | Before | After |
|---|---|---|---|
| `anodeMaterial` | added | — | `array\|null` |
| `batteryType` | added | — | `string\|null` |
| `batteryWeightKg` | added | — | `number\|null` |
| `carbonFootprintClass` | added | — | `string\|null` |
| `cathodeMaterial` | added | — | `array\|null` |
| `criticalRawMaterials` | added | — | `array\|null` |
| `disassemblyInstructionsUrl` | added | — | `string\|null` |
| `dueDiligenceUrl` | added | — | `string\|null` |
| `electrolyteMaterial` | added | — | `array\|null` |
| `internalResistanceMohm` | added | — | `number\|null` |
| `operatingTempMaxC` | added | — | `number\|null` |
| `operatingTempMinC` | added | — | `number\|null` |
| `ratedEnergyWh` | added | — | `number\|null` |
| `recycledContentLeadPct` | added | — | `number\|null` |
| `roundTripEfficiencyPct` | added | — | `number\|null` |
| `sohMethodology` | added | — | `string\|null` |

### v2.0.0 → v2.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `carbonFootprintClass` | constraint changed | `enum=["A","B","C","D","E",null]` | — |
| `carbonFootprintClass` | constraint changed | — | `maxLength=8` |
| `carbonFootprintClassRulesetId` | added | — | `string\|null` |
| `carbonFootprintClassRulesetVersion` | added | — | `string\|null` |
| `placedOnMarketDate` | added | — | `string\|null` |

### v2.1.0 → v2.2.0 · additive

| Property | Change | Before | After |
|---|---|---|---|
| `stateOfHealth` | added | — | — |

### v2.2.0 → v2.3.0 · additive

| Property | Change | Before | After |
|---|---|---|---|
| `recycledContentReportingYear` | added | — | `integer\|null` |

### v2.3.0 → v2.4.0 · additive

| Property | Change | Before | After |
|---|---|---|---|
| `expectedLifetime` | added | — | `object\|null` |

### v2.4.0 → v2.5.0

| Property | Change | Before | After |
|---|---|---|---|
| `batteryType` | type changed | `string\|null` | `string` |
| `batteryType` | constraint changed | `enum=["portable","industrial","ev","lmt","starting-lighting-ignition",null]` | `enum=["portable","industrial","ev","lmt","starting-lighting-ignition"]` |

**Newly required:** `batteryType`

### v2.5.0 → v2.6.0

| Property | Change | Before | After |
|---|---|---|---|
| `batteryModelId` | added | — | `string\|null` |
| `batteryPassportNumber` | added | — | `string\|null` |
| `batteryStatus` | added | — | `string\|null` |
| `capacityThresholdForExhaustionPct` | added | — | `number\|null` |
| `commercialWarrantyPeriodMonths` | added | — | `integer\|null` |
| `componentPartNumbers` | added | — | `array\|null` |
| `cycleLifeTestCRate` | added | — | `number\|null` |
| `dynamicPerformance` | added | — | `object\|null` |
| `euDeclarationOfConformity` | added | — | `string\|null` |
| `expectedLifetimeCycles` | type changed | `integer` | `integer\|null` |
| `expectedLifetimeReferenceTest` | added | — | `string\|null` |
| `hazardSymbol` | added | — | `string\|null` |
| `hazardousSubstances` | added | — | `array\|null` |
| `initialRoundTripEfficiencyPct` | added | — | `number\|null` |
| `internalCellResistanceMohm` | added | — | `number\|null` |
| `internalPackResistanceMohm` | added | — | `number\|null` |
| `manufacturingDate` | added | — | `string\|null` |
| `manufacturingPlace` | added | — | `string\|null` |
| `markingInformation` | added | — | `string\|null` |
| `maximumVoltageV` | added | — | `number\|null` |
| `minimalVoltageV` | added | — | `number\|null` |
| `notInUseTemperatureRange` | added | — | `object\|null` |
| `notInUseTemperatureReferenceTest` | added | — | `string\|null` |
| `originalPowerCapabilityW` | added | — | `number\|null` |
| `powerLimitMaxW` | added | — | `number\|null` |
| `powerLimitMinW` | added | — | `number\|null` |
| `powerTemperatureRange` | added | — | `object\|null` |
| `renewableContentPct` | added | — | `number\|null` |
| `roundTripEfficiencyAtHalfCycleLifePct` | added | — | `number\|null` |
| `roundTripEfficiencyPct` | constraint changed | `maximum=100` | — |
| `safetyMeasures` | added | — | `string\|null` |
| `sparePartsContacts` | added | — | `string\|null` |
| `testReportResults` | added | — | `string\|null` |
| `usableExtinguishingAgent` | added | — | `string\|null` |
| `usageHistory` | added | — | `object\|null` |
| `voltageTemperatureRange` | added | — | `object\|null` |
| `wasteBatteryInformation` | added | — | `string\|null` |

**No longer required:** `expectedLifetimeCycles`

## construction


### v1.0.0 → v1.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `countryOfManufacture` | removed | `string` | — |
| `countryOfOrigin` | added | — | `string` |

**Newly required:** `countryOfOrigin`

**No longer required:** `countryOfManufacture`

## detergent


### v1.0.0 → v1.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `countryOfManufacture` | removed | `string` | — |
| `countryOfOrigin` | added | — | `string` |

**Newly required:** `countryOfOrigin`

**No longer required:** `countryOfManufacture`

## electronics


### v1.0.0 → v1.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `repairabilityScore` | type changed | `number\|null` | `object\|null` |
| `repairabilityScore` | constraint changed | `maximum=10` | — |
| `repairabilityScore` | constraint changed | `minimum=0` | — |
| `repairabilityScore/criteria` | added | — | `array` |
| `repairabilityScore/overall` | added | — | `number` |

### v1.1.0 → v1.2.0

| Property | Change | Before | After |
|---|---|---|---|
| `productCategory` | constraint changed | `enum=["smartphone","laptop","tablet","monitor","tv","server","router","charger","earphone","pcb","other"]` | `enum=["smartphone","other-mobile-phone","cordless-phone","tablet"]` |

## furniture


### v1.0.0 → v1.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `countryOfManufacture` | removed | `string` | — |
| `countryOfOrigin` | added | — | `string` |

**Newly required:** `countryOfOrigin`

**No longer required:** `countryOfManufacture`

### v1.1.0 → v1.2.0

| Property | Change | Before | After |
|---|---|---|---|
| `productType` | constraint changed | `enum=["chair","table","sofa","mattress","shelf","bed","other"]` | `enum=["chair","table","sofa","shelf","bed","other"]` |

## mattress

Only one version (v1.0.0) — nothing to compare against yet.

## steel


### v1.0.0 → v1.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `countryOfOrigin` | added | — | `string` |
| `countryOfProduction` | removed | `string` | — |

**Newly required:** `countryOfOrigin`

**No longer required:** `countryOfProduction`

## textile


### v1.0.0 → v1.1.0 · additive

| Property | Change | Before | After |
|---|---|---|---|
| `allergens` | added | — | `array\|null` |
| `disassemblyInstructions` | added | — | `string\|null` |
| `durabilityScore` | added | — | `number\|null` |
| `endOfLifeInstructions` | added | — | `string\|null` |
| `expectedWashCycles` | added | — | `integer\|null` |
| `fibreComposition/[]/countryOfOrigin` | added | — | `string` |
| `microplasticSheddingMgPerWash` | added | — | `number\|null` |
| `pefScore` | added | — | `number\|null` |
| `priorUseCycles` | added | — | `integer\|null` |
| `productWeightGrams` | added | — | `number\|null` |
| `recyclabilityClass` | added | — | `string\|null` |
| `repairCount` | added | — | `integer\|null` |
| `repairHistoryUrl` | added | — | `string\|null` |
| `reuseCondition` | added | — | `string\|null` |
| `sparePartsAvailable` | added | — | `boolean\|null` |
| `substancesOfConcern` | added | — | `array\|null` |
| `svhcSubstances` | added | — | `array\|null` |

### v1.1.0 → v1.2.0

| Property | Change | Before | After |
|---|---|---|---|
| `countryOfManufacturing` | removed | `string` | — |
| `countryOfOrigin` | added | — | `string` |

**Newly required:** `countryOfOrigin`

**No longer required:** `countryOfManufacturing`

## toy


### v1.0.0 → v1.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `countryOfManufacture` | removed | `string` | — |
| `countryOfOrigin` | added | — | `string` |

**Newly required:** `countryOfOrigin`

**No longer required:** `countryOfManufacture`

## tyre

Only one version (v1.0.0) — nothing to compare against yet.

## unsold-goods

Only one version (v2.0.0) — nothing to compare against yet.

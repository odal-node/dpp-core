# Schema changes

**Generated: do not edit.** Regenerate with `just schema-changes`;
`cargo test -p dpp-tests --test schema_changes` fails if this file has
drifted from the schemas under `crates/dpp-domain/schemas/`.

One section per product group, one table per version bump: what each
version changed relative to the one before it.

`additive` means the bump only adds properties: no removal, no type
change, no altered constraint, no newly required property. It is a
statement about *shape* and not a compatibility verdict; whether stored
documents still read is answered by the frozen fixtures in
`schema_compat.rs`, which test it rather than infer it.

## aluminium


### v1.0.0 → v1.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `countryOfOrigin` | added | absent | `string` |
| `countryOfProduction` | removed | `string` | absent |

**Newly required:** `countryOfOrigin`

**No longer required:** `countryOfProduction`

### v1.1.0 → v1.2.0

| Property | Change | Before | After |
|---|---|---|---|
| `gtin` | removed | `string` | absent |
| `productIdentifier` | added | absent | `object` |

**Newly required:** `productIdentifier`

**No longer required:** `gtin`

## battery


### v1.0.0 → v2.0.0 · additive

| Property | Change | Before | After |
|---|---|---|---|
| `anodeMaterial` | added | absent | `array\|null` |
| `batteryType` | added | absent | `string\|null` |
| `batteryWeightKg` | added | absent | `number\|null` |
| `carbonFootprintClass` | added | absent | `string\|null` |
| `cathodeMaterial` | added | absent | `array\|null` |
| `criticalRawMaterials` | added | absent | `array\|null` |
| `disassemblyInstructionsUrl` | added | absent | `string\|null` |
| `dueDiligenceUrl` | added | absent | `string\|null` |
| `electrolyteMaterial` | added | absent | `array\|null` |
| `internalResistanceMohm` | added | absent | `number\|null` |
| `operatingTempMaxC` | added | absent | `number\|null` |
| `operatingTempMinC` | added | absent | `number\|null` |
| `ratedEnergyWh` | added | absent | `number\|null` |
| `recycledContentLeadPct` | added | absent | `number\|null` |
| `roundTripEfficiencyPct` | added | absent | `number\|null` |
| `sohMethodology` | added | absent | `string\|null` |

### v2.0.0 → v2.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `carbonFootprintClass` | constraint changed | `enum=["A","B","C","D","E",null]` | absent |
| `carbonFootprintClass` | constraint changed | absent | `maxLength=8` |
| `carbonFootprintClassRulesetId` | added | absent | `string\|null` |
| `carbonFootprintClassRulesetVersion` | added | absent | `string\|null` |
| `placedOnMarketDate` | added | absent | `string\|null` |

### v2.1.0 → v2.2.0 · additive

| Property | Change | Before | After |
|---|---|---|---|
| `stateOfHealth` | added | absent | absent |

### v2.2.0 → v2.3.0 · additive

| Property | Change | Before | After |
|---|---|---|---|
| `recycledContentReportingYear` | added | absent | `integer\|null` |

### v2.3.0 → v2.4.0 · additive

| Property | Change | Before | After |
|---|---|---|---|
| `expectedLifetime` | added | absent | `object\|null` |

### v2.4.0 → v2.5.0

| Property | Change | Before | After |
|---|---|---|---|
| `batteryType` | type changed | `string\|null` | `string` |
| `batteryType` | constraint changed | `enum=["portable","industrial","ev","lmt","starting-lighting-ignition",null]` | `enum=["portable","industrial","ev","lmt","starting-lighting-ignition"]` |

**Newly required:** `batteryType`

### v2.5.0 → v2.6.0

| Property | Change | Before | After |
|---|---|---|---|
| `batteryModelId` | added | absent | `string\|null` |
| `batteryPassportNumber` | added | absent | `string\|null` |
| `batteryStatus` | added | absent | `string\|null` |
| `capacityThresholdForExhaustionPct` | added | absent | `number\|null` |
| `commercialWarrantyPeriodMonths` | added | absent | `integer\|null` |
| `componentPartNumbers` | added | absent | `array\|null` |
| `cycleLifeTestCRate` | added | absent | `number\|null` |
| `dynamicPerformance` | added | absent | `object\|null` |
| `euDeclarationOfConformity` | added | absent | `string\|null` |
| `expectedLifetimeCycles` | type changed | `integer` | `integer\|null` |
| `expectedLifetimeReferenceTest` | added | absent | `string\|null` |
| `hazardSymbol` | added | absent | `string\|null` |
| `hazardousSubstances` | added | absent | `array\|null` |
| `initialRoundTripEfficiencyPct` | added | absent | `number\|null` |
| `internalCellResistanceMohm` | added | absent | `number\|null` |
| `internalPackResistanceMohm` | added | absent | `number\|null` |
| `manufacturingDate` | added | absent | `string\|null` |
| `manufacturingPlace` | added | absent | `string\|null` |
| `markingInformation` | added | absent | `string\|null` |
| `maximumVoltageV` | added | absent | `number\|null` |
| `minimalVoltageV` | added | absent | `number\|null` |
| `notInUseTemperatureRange` | added | absent | `object\|null` |
| `notInUseTemperatureReferenceTest` | added | absent | `string\|null` |
| `originalPowerCapabilityW` | added | absent | `number\|null` |
| `powerLimitMaxW` | added | absent | `number\|null` |
| `powerLimitMinW` | added | absent | `number\|null` |
| `powerTemperatureRange` | added | absent | `object\|null` |
| `renewableContentPct` | added | absent | `number\|null` |
| `roundTripEfficiencyAtHalfCycleLifePct` | added | absent | `number\|null` |
| `roundTripEfficiencyPct` | constraint changed | `maximum=100` | absent |
| `safetyMeasures` | added | absent | `string\|null` |
| `sparePartsContacts` | added | absent | `string\|null` |
| `testReportResults` | added | absent | `string\|null` |
| `usableExtinguishingAgent` | added | absent | `string\|null` |
| `usageHistory` | added | absent | `object\|null` |
| `voltageTemperatureRange` | added | absent | `object\|null` |
| `wasteBatteryInformation` | added | absent | `string\|null` |

**No longer required:** `expectedLifetimeCycles`

### v2.6.0 → v2.7.0

| Property | Change | Before | After |
|---|---|---|---|
| `gtin` | removed | `string` | absent |
| `productIdentifier` | added | absent | `object` |

**Newly required:** `productIdentifier`

**No longer required:** `gtin`

### v2.7.0 → v2.8.0


**No longer required:** `co2ePerUnitKg`

### v2.8.0 → v2.9.0 · additive

No property or requirement changed.

### v2.9.0 → v2.10.0 · additive

| Property | Change | Before | After |
|---|---|---|---|
| `dueDiligenceOperatorScope` | added | absent | `string\|null` |

## construction


### v1.0.0 → v1.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `countryOfManufacture` | removed | `string` | absent |
| `countryOfOrigin` | added | absent | `string` |

**Newly required:** `countryOfOrigin`

**No longer required:** `countryOfManufacture`

### v1.1.0 → v1.2.0

| Property | Change | Before | After |
|---|---|---|---|
| `gtin` | removed | `string` | absent |
| `productIdentifier` | added | absent | `object` |

**Newly required:** `productIdentifier`

**No longer required:** `gtin`

## detergent


### v1.0.0 → v1.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `countryOfManufacture` | removed | `string` | absent |
| `countryOfOrigin` | added | absent | `string` |

**Newly required:** `countryOfOrigin`

**No longer required:** `countryOfManufacture`

### v1.1.0 → v1.2.0

| Property | Change | Before | After |
|---|---|---|---|
| `gtin` | removed | `string` | absent |
| `productIdentifier` | added | absent | `object` |

**Newly required:** `productIdentifier`

**No longer required:** `gtin`

## electronics


### v1.0.0 → v1.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `repairabilityScore` | type changed | `number\|null` | `object\|null` |
| `repairabilityScore` | constraint changed | `maximum=10` | absent |
| `repairabilityScore` | constraint changed | `minimum=0` | absent |
| `repairabilityScore/criteria` | added | absent | `array` |
| `repairabilityScore/overall` | added | absent | `number` |

### v1.1.0 → v1.2.0

| Property | Change | Before | After |
|---|---|---|---|
| `productCategory` | constraint changed | `enum=["smartphone","laptop","tablet","monitor","tv","server","router","charger","earphone","pcb","other"]` | `enum=["smartphone","other-mobile-phone","cordless-phone","tablet"]` |

### v1.2.0 → v1.3.0 · additive

| Property | Change | Before | After |
|---|---|---|---|
| `indexScopeExclusion` | added | absent | `string\|null` |
| `repairabilityIndexInputs` | added | absent | `object\|null` |

### v1.3.0 → v1.4.0

| Property | Change | Before | After |
|---|---|---|---|
| `gtin` | removed | `string` | absent |
| `productIdentifier` | added | absent | `object` |

**Newly required:** `productIdentifier`

**No longer required:** `gtin`

### v1.4.0 → v1.5.0 · additive

No property or requirement changed.

## furniture


### v1.0.0 → v1.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `countryOfManufacture` | removed | `string` | absent |
| `countryOfOrigin` | added | absent | `string` |

**Newly required:** `countryOfOrigin`

**No longer required:** `countryOfManufacture`

### v1.1.0 → v1.2.0

| Property | Change | Before | After |
|---|---|---|---|
| `productType` | constraint changed | `enum=["chair","table","sofa","mattress","shelf","bed","other"]` | `enum=["chair","table","sofa","shelf","bed","other"]` |

### v1.2.0 → v1.3.0

| Property | Change | Before | After |
|---|---|---|---|
| `gtin` | removed | `string` | absent |
| `productIdentifier` | added | absent | `object` |

**Newly required:** `productIdentifier`

**No longer required:** `gtin`

## mattress


### v1.0.0 → v1.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `gtin` | removed | `string` | absent |
| `productIdentifier` | added | absent | `object` |

**Newly required:** `productIdentifier`

**No longer required:** `gtin`

## steel


### v1.0.0 → v1.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `countryOfOrigin` | added | absent | `string` |
| `countryOfProduction` | removed | `string` | absent |

**Newly required:** `countryOfOrigin`

**No longer required:** `countryOfProduction`

### v1.1.0 → v1.2.0

| Property | Change | Before | After |
|---|---|---|---|
| `gtin` | removed | `string` | absent |
| `productIdentifier` | added | absent | `object` |

**Newly required:** `productIdentifier`

**No longer required:** `gtin`

## textile


### v1.0.0 → v1.1.0 · additive

| Property | Change | Before | After |
|---|---|---|---|
| `allergens` | added | absent | `array\|null` |
| `disassemblyInstructions` | added | absent | `string\|null` |
| `durabilityScore` | added | absent | `number\|null` |
| `endOfLifeInstructions` | added | absent | `string\|null` |
| `expectedWashCycles` | added | absent | `integer\|null` |
| `fibreComposition/[]/countryOfOrigin` | added | absent | `string` |
| `microplasticSheddingMgPerWash` | added | absent | `number\|null` |
| `pefScore` | added | absent | `number\|null` |
| `priorUseCycles` | added | absent | `integer\|null` |
| `productWeightGrams` | added | absent | `number\|null` |
| `recyclabilityClass` | added | absent | `string\|null` |
| `repairCount` | added | absent | `integer\|null` |
| `repairHistoryUrl` | added | absent | `string\|null` |
| `reuseCondition` | added | absent | `string\|null` |
| `sparePartsAvailable` | added | absent | `boolean\|null` |
| `substancesOfConcern` | added | absent | `array\|null` |
| `svhcSubstances` | added | absent | `array\|null` |

### v1.1.0 → v1.2.0

| Property | Change | Before | After |
|---|---|---|---|
| `countryOfManufacturing` | removed | `string` | absent |
| `countryOfOrigin` | added | absent | `string` |

**Newly required:** `countryOfOrigin`

**No longer required:** `countryOfManufacturing`

### v1.2.0 → v1.3.0

| Property | Change | Before | After |
|---|---|---|---|
| `gtin` | removed | `string` | absent |
| `productIdentifier` | added | absent | `object` |

**Newly required:** `productIdentifier`

**No longer required:** `gtin`

### v1.3.0 → v1.4.0 · additive

No property or requirement changed.

## toy


### v1.0.0 → v1.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `countryOfManufacture` | removed | `string` | absent |
| `countryOfOrigin` | added | absent | `string` |

**Newly required:** `countryOfOrigin`

**No longer required:** `countryOfManufacture`

### v1.1.0 → v1.2.0

| Property | Change | Before | After |
|---|---|---|---|
| `gtin` | removed | `string` | absent |
| `productIdentifier` | added | absent | `object` |

**Newly required:** `productIdentifier`

**No longer required:** `gtin`

## tyre


### v1.0.0 → v1.1.0

| Property | Change | Before | After |
|---|---|---|---|
| `gtin` | removed | `string` | absent |
| `productIdentifier` | added | absent | `object` |

**Newly required:** `productIdentifier`

**No longer required:** `gtin`

## unsold-goods

Only one version (v2.0.0), so there is nothing to compare against yet.

# IRONROOT SynthXML Schema Draft

This document defines the first practical SynthXML schema for IRONROOT.

It is intentionally simple enough to hand-edit while preserving enough structure for deterministic compilation.

---

## Root

```xml
<ironroot-synth score-id="string" version="1">
```

Attributes:

| Attribute | Required | Meaning |
|---|---:|---|
| `score-id` | yes | Stable ID |
| `version` | yes | Schema version |

---

## Metadata

```xml
<metadata>
  <source type="musicxml" hash="u64hex"/>
  <title>Recovered Tavern Fragment</title>
  <origin>original</origin>
  <tempo bpm-x100="9200"/>
  <mode name="minor"/>
  <world-seed-affinity enabled="true"/>
</metadata>
```

---

## Analysis

```xml
<analysis>
  <account-pressure account="LastToll" q="6200"/>
  <quincunx demand-a="WitnessBuilding" demand-b="Mobility" severity-q="3500"/>
  <yod base-a-q="2000" base-b-q="2600" apex-resilience-q="1800" cooperation-q="1200"/>
  <finger-of-god q="5400"/>
  <first-lock-hint id="9" secrecy-q="8000"/>
</analysis>
```

---

## Threads

```xml
<threads>
  <thread id="bell_main" type="BellThread" account="LastToll" loop-ticks="184320"/>
  <thread id="voice_memory" type="WitnessThread" account="GraveWater" loop-ticks="276480"/>
</threads>
```

Thread attributes:

| Attribute | Required | Meaning |
|---|---:|---|
| `id` | yes | Local stable thread ID |
| `type` | yes | Thread type |
| `account` | yes | Hidden account name |
| `loop-ticks` | no | Symbolic loop length |

Allowed thread types:

```text
BellThread
FolkMelodyThread
DroneThread
WitnessThread
HollowThread
RouteThread
SilenceThread
DjPulseThread
BroadcastThread
```

---

## Events

```xml
<events>
  <note thread="bell_main" t="0" dur="960" pitch="64" velocity-q="7000" pressure-q="3000"/>
  <rest thread="bell_main" t="960" dur="480"/>
  <silence thread="hollow_gap" t="3840" dur="960" reason="MissingTonic"/>
  <primitive thread="voice_memory" t="7680" id="choir_residue_soft" amount-q="4200"/>
  <cadence thread="bell_main" t="12000" kind="RecursiveToll"/>
  <ledger-marker thread="voice_memory" t="14000" hash="ab12"/>
</events>
```

---

## Faust

```xml
<faust>
  <processor id="bell_memory_filter" thread="bell_main"/>
  <processor id="hollow_tonic_remover" thread="hollow_gap"/>
</faust>
```

---

## Example Complete File

```xml
<ironroot-synth score-id="last_toll_fragment_001" version="1">
  <metadata>
    <source type="musicxml" hash="a36fe20199"/>
    <title>The Bell Not Rung</title>
    <origin>original</origin>
    <tempo bpm-x100="7200"/>
    <mode name="minor"/>
    <world-seed-affinity enabled="true"/>
  </metadata>

  <analysis>
    <account-pressure account="LastToll" q="7800"/>
    <account-pressure account="HollowStar" q="2100"/>
    <quincunx demand-a="StationaryChannel" demand-b="Theft" severity-q="5200"/>
    <yod base-a-q="2200" base-b-q="2400" apex-resilience-q="1100" cooperation-q="1600"/>
    <finger-of-god q="6100"/>
    <first-lock-hint id="9" secrecy-q="9000"/>
  </analysis>

  <threads>
    <thread id="bell" type="BellThread" account="LastToll" loop-ticks="69120"/>
    <thread id="room" type="DroneThread" account="StoneRoot" loop-ticks="184320"/>
    <thread id="missing" type="SilenceThread" account="HollowStar" loop-ticks="276480"/>
  </threads>

  <events>
    <note thread="bell" t="0" dur="960" pitch="55" velocity-q="7000" pressure-q="5000"/>
    <rest thread="bell" t="960" dur="1920"/>
    <note thread="bell" t="2880" dur="960" pitch="55" velocity-q="6400" pressure-q="5200"/>
    <silence thread="missing" t="3840" dur="960" reason="MissingTonic"/>
    <cadence thread="bell" t="5760" kind="RecursiveToll"/>
    <primitive thread="room" t="0" id="wood_room_low" amount-q="4500"/>
  </events>

  <faust>
    <processor id="bell_memory_filter" thread="bell"/>
    <processor id="last_toll_recursion" thread="bell"/>
    <processor id="hollow_tonic_remover" thread="missing"/>
  </faust>
</ironroot-synth>
```

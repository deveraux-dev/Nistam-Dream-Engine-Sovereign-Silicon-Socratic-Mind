# Rhythm Combat, Synthesia, MusicXML, and MIDI 2.0 Mapping

## Core mapping

```text
Synthesia:
  time -> falling note -> lane -> hit moment -> feedback

Dirge:
  time -> incoming intent -> lane -> action window -> player response
```

## Combat lanes

```text
C = block
D = dodge
E = parry
F = strike
G = relic
```

## MIDI-style event interpretation

| MIDI concept | Dirge meaning |
|---|---|
| note_on | attack telegraph begins |
| pitch | lane / height / body target |
| velocity | attack force / threat |
| duration | active window |
| channel/group | enemy section / faction |
| aftertouch | pressure / held curse / shield strain |
| pitch bend | attack angle / sweep curve |
| controller | poison depth / burn intensity / stagger pull |
| release velocity | recovery danger / recoil |

## MusicXML structure mapping

| MusicXML concept | Dirge meaning |
|---|---|
| part | enemy section / arena system |
| measure | tactical phrase |
| note | attack / cue / spawn / hazard |
| duration | active window |
| dynamic | threat intensity |
| articulation | behavior type |
| slur | linked combo chain |
| rest | safety window |
| rehearsal mark | checkpoint / phase marker |
| crescendo | escalating threat |
| fermata | hold / suspense / delayed release |

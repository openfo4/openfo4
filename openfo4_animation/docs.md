- Events are used to send data between game and hkx data. For example the passed NPCRobotTurretLowTechMotorLPM part of
an event payload is a SNDR record.
- Events are probably recognized by the game by name (e.g. `PlaySound`).
- Variables are probably also recognized/injected by the game.

- transitions and wildcard_transitions are triggered when the event with `eventId` is received and transition to
`toStateId`.

- Modifiers and generators are essentially the same thing, they do something when they become
active?
- `startStateId` is not an index, but the `stateId` value of the `StateMachineStateInfo`.
- `wildcardTransitions` is directly set on the `StateMachine` and will define a set of transitions that are always valid.
- `transitions` is set on `StateMachineStateInfo` and will define a set of local transitions that are only valid
while the `StateMachine` is this state.
- `VariableBindingSet` can be used to overwrite class fields with runtime provided variables/properties.

- `BlendingTransitionEffect`: Blends between active animations for the given duration.
- `GeneratorTransitionEffect`: Enables the generator for the transition, then disables when transition is done? For how long is the generator activated? The time of the animation? What if the generator is a state machine or does never complete?

## Nodes

### BSAssignVariablesModifier

Used to set variables. This node contains a set of `(float|int)Variable*` and `(float|int)Value*` tuples.

- `variableBindingSet` is used to set `Variable` and `Value` fields.
- If `(float|int)Variable*` is not `0` (int) or `0.0` (float) the corresponding `Value` field is copied
into the variable index specified by `Variable`.
- It is probably best to ignored the `Variable` fields completely and only copy if the field contains a binding.
- `floatVariable` is used as indices for variables? Why not int?

### BSIsActiveModifier

- Used to set a bool variable to `true` if the current node is active?

### BSDirectAtModifier

- Changes bones with camera?


### ClipGenerator

Leaf node that plays animations.


## List of game events

| Name        | Payload | |
| ----------- | ------- | - |
| `PlaySound` | String  | Starts playing a sound. The payload contains the Editor ID (`EDID`) of the `SNDR` record. |

- `PlaySound`
- `StopSound`

- `attackStart`
- `attackEnd`
- `attackStartAuto`
- `attackStop`
- `attackRelease`

`g_weapForceEquip` referenced in Fallout4.esm? `IDLE` form. Seems like `IDLE` forms somehow ,





## List of game variables

- `bAimActive`
- `bAimEnabled`
- `IsAttackReady`
- `bEquipOk`
- `bAnimationDriven`
- `AimPitchMaxUp`
- `AimPitchMaxDown`
- `AimPitchCurrent`
- `weaponSpeedMult`
- `AimHeadingMaxCCW`
- `AimHeadingMaxCW`
- `AimHeadingCurrent`

## Animation

- `Annotationtrack` contains extra events that can be sent to the game (e.g. `SoundPlay.NPCRobotTurretHighTechInjuredCritical`)

# Skinning

- `RACE` contains path of skeleton nif file (`ANAM`) and reference to body part data (`GNAM`).
- Nif skeleton is overlayed over the same hkx skeleton based on bone names, case insensitive?
- Body part data references nif skeleton bone where the model should be mounted based on name.
- Bonus: The Skin nif contains a `AttachT` node with the name of the attached bone?

- `RACE` links to Skin (`ARMO`), which links to armor addons with skinned Nifs (`ARMA`).

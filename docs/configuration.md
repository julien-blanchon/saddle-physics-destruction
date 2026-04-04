# Configuration

## `DestructionConfig`

| Field | Type | Default | Recommended Range | Effect | Tradeoff |
|------|------|---------|-------------------|--------|----------|
| `fragment_budget` | `usize` | `256` | `32..512` | Global cap on surviving fragment entities after cleanup | Lower values reduce entity pressure but trim debris more aggressively |
| `cleanup_policy` | `CleanupPolicy` | `OldestFirst` | Per content type | Chooses which fragments are removed first when over budget | Different policies favor stability, readability, or density |
| `default_fragment_lifetime_secs` | `f32` | `8.0` | `2.0..12.0` | Base lifetime given to spawned fragments | Longer lifetimes keep debris readable but grow entity count |
| `fragment_fade_secs` | `f32` | `1.25` | `0.0..2.0` | Fade window at the end of fragment lifetime | Zero disables fade and removes fragments abruptly |
| `max_fragment_distance` | `f32` | `80.0` | `20.0..120.0` | Distance from the nearest viewer before debris is culled | Tight distances improve perf but can pop visible debris |
| `max_chunk_spawns_per_frame` | `usize` | `48` | `4..64` | Per-frame activation throttle for detached groups; excess fragment spawns stay queued | Lower values smooth spikes but delay full breakage |
| `enable_support_evaluation` | `bool` | `true` | `true` for structural props | Enables flood-fill support collapse after topology changes | Disabling it leaves only direct chunk/bond detachment |
| `inherit_velocity` | `bool` | `true` | Either | Uses incoming hit direction when computing fragment velocity hints | Disabling produces more neutral detachment motion |
| `distance_lod` | `Vec<DistanceLodBand>` | `[Full to infinity]` | `1..4` bands | Distance-based activation policy | Lets far targets degrade to coarser or event-only output |

## `CleanupPolicy`

| Variant | Behavior |
|---------|----------|
| `OldestFirst` | Remove the fragments with the least remaining lifetime first |
| `SmallestFirst` | Remove the smallest approximate fragments first |
| `FarthestFirst` | Remove the farthest surviving fragments first |

## `DistanceLodBand`

| Field | Type | Meaning |
|------|------|---------|
| `max_distance` | `f32` | Inclusive upper bound for this band |
| `strategy` | `LodStrategy` | Activation behavior when the nearest viewer is within this band |

## `LodStrategy`

| Variant | Effect |
|---------|--------|
| `Full` | Spawn all selected fine chunks |
| `Clustered { minimum_leaf_count }` | Prefer authored parent chunks when enough leaves can be represented by a larger group |
| `EventOnly` | Skip fragment entity spawning and emit only detachment messages |

## `Destructible`

| Field | Type | Default | Recommended Range | Effect |
|------|------|---------|-------------------|--------|
| `visual_mode` | `RootVisualMode` | `HideOnFirstDetach` | Per content type | Controls when the intact/root presentation should be hidden |

## `RootVisualMode`

| Variant | Effect |
|---------|--------|
| `KeepVisible` | Never hide the intact root automatically |
| `HideOnFirstDetach` | Hide as soon as any chunk detaches |
| `HideWhenBroken` | Keep visible until all support chunks are detached |

## `SupportAnchors`

| Field | Type | Default | Recommended Range | Effect |
|------|------|---------|-------------------|--------|
| `chunks` | `Vec<ChunkId>` | empty | `0..support_chunk_count` | Replaces authored anchor selection for that entity when non-empty |

## `DestructionViewers`

| Field | Type | Default | Recommended Range | Effect |
|------|------|---------|-------------------|--------|
| `positions` | `Vec<Vec3>` | empty | `1..4` active viewers | Viewer positions used for LOD selection and distance culling |

If no viewers are present, the runtime falls back to near/full behavior and does not distance-trim debris.

## `DestructionEffectHooks`

| Field | Type | Default | Recommended Range | Effect |
|------|------|---------|-------------------|--------|
| `start_audio_cue` | `Option<String>` | `None` | Project-specific cue id | Audio cue emitted when the destructible first enters an active damage state |
| `start_particle_cue` | `Option<String>` | `None` | Project-specific cue id | Particle cue emitted with the same first-damage transition |
| `detach_audio_cue` | `Option<String>` | `None` | Project-specific cue id | Audio cue emitted when a detached chunk group activates |
| `detach_particle_cue` | `Option<String>` | `None` | Project-specific cue id | Particle cue emitted when a detached chunk group activates |
| `final_audio_cue` | `Option<String>` | `None` | Project-specific cue id | Audio cue emitted when the root reaches the fully broken state |
| `final_particle_cue` | `Option<String>` | `None` | Project-specific cue id | Particle cue emitted when the root reaches the fully broken state |

If a root has this component, the runtime emits `DestructionEffectTriggered` messages that already include the selected audio/particle cue ids plus stage, material, world position, energy, and detached-fragment counts.

## `DestructionAvianFragments` (`avian3d` feature)

| Field | Type | Default | Recommended Range | Effect |
|------|------|---------|-------------------|--------|
| `rigid_body` | `RigidBody` | `Dynamic` | Usually `Dynamic` | Rigid-body mode inserted on spawned fragments |
| `mass_scale` | `f32` | `1.0` | `0.25..2.0` | Scales the authored `mass_hint` before inserting `Mass` |
| `linear_damping` | `f32` | `0.12` | `0.0..1.0` | Linear damping applied to spawned fragments |
| `angular_damping` | `f32` | `0.2` | `0.0..1.0` | Angular damping applied to spawned fragments |
| `gravity_scale` | `f32` | `1.0` | `0.0..2.0` | Per-fragment gravity multiplier |
| `friction` | `f32` | `0.8` | `0.0..2.0` | Friction coefficient inserted on fragments |
| `restitution` | `f32` | `0.08` | `0.0..1.0` | Bounce coefficient inserted on fragments |
| `collision_layers` | `Option<CollisionLayers>` | `None` | Project-specific mask | Optional collision-layer override inserted on fragments |

Attach this component to a destructible root only when the crate is compiled with `features = ["avian3d"]`.

## `DestructionDebugConfig`

| Field | Type | Default | Recommended Range | Effect |
|------|------|---------|-------------------|--------|
| `draw_support_graph` | `bool` | `false` | Either | Draw bond/support relationships |
| `draw_support_anchors` | `bool` | `false` | Either | Draw anchor chunk markers |
| `draw_unsupported_groups` | `bool` | `false` | Either | Draw unsupported-island or floating-detach group hints |
| `draw_last_damage` | `bool` | `false` | Either | Visualize the most recent recorded hit |

## Damage Input

### `ApplyDestructionDamage`

| Field | Type | Meaning |
|------|------|---------|
| `target` | `Entity` | Destructible entity to damage |
| `origin` | `Vec3` | Hit origin in world space |
| `direction` | `Vec3` | Incoming world-space direction or bias axis |
| `magnitude` | `f32` | Overall damage strength |
| `radius` | `f32` | Radius used by falloff and directional weighting |
| `kind` | `DamageKind` | Point vs radial vs directional vs shear interpretation |
| `falloff` | `FalloffCurve` | Distance attenuation model |
| `fracture_bias` | `FractureBias` | Coarse, balanced, or fine activation preference |

### `DamageKind`

| Variant | Effect |
|---------|--------|
| `Point` | Slightly favors concentrated local damage |
| `Radial` | Symmetric radius-based damage |
| `Directional` | Favors chunks and bonds aligned with the incoming direction |
| `Shear` | Favors lateral separation and bond normal mismatch |

### `FalloffCurve`

| Variant | Effect |
|---------|--------|
| `Constant` | No distance attenuation |
| `Linear` | Straight-line falloff |
| `SmoothStep` | Softer center-to-edge falloff |
| `Quadratic` | Stronger edge attenuation |

### `FractureBias`

| Variant | Effect |
|---------|--------|
| `Coarse` | Prefer broader breakage / larger represented chunks when the LOD path allows it |
| `Balanced` | Default compromise between fine and coarse activation |
| `Fine` | Prefer finer chunk activation |

## Authoring Builders

### `CuboidFractureBuilder`

| Field | Type | Default | Recommended Range | Effect |
|------|------|---------|-------------------|--------|
| `size` | `Vec3` | required | Positive authored bounds | Overall authored bounds |
| `cells` | `UVec3` | required | `1..8` per axis for interactive labs | Leaf subdivision resolution |
| `coarse_groups` | `Option<UVec3>` | `None` | `None` or divisors of `cells` | Optional authored parent grouping grid |
| `seed` | `u64` | `1` | Any non-zero deterministic seed | Deterministic split jitter seed |
| `jitter` | `f32` | `0.2` | `0.0..0.49` | Split offset amount along each axis |
| `material_hint` | `MaterialHint` | `Concrete` | Per content type | Material preset propagated to chunks |
| `anchor_preset` | `CuboidAnchorPreset` | `Bottom` | Per structure type | Built-in anchor placement |

### `ThinSurfaceFractureBuilder`

| Field | Type | Default | Recommended Range | Effect |
|------|------|---------|-------------------|--------|
| `size` | `Vec2` | required | Positive authored bounds | Panel width/height |
| `depth` | `f32` | required | `0.01..0.25` | Extrusion thickness |
| `cells` | `usize` | required | `4..64` | Number of Voronoi-like cells to generate |
| `seed` | `u64` | `7` | Any non-zero deterministic seed | Deterministic site placement seed |
| `anchor_preset` | `ThinSurfaceAnchorPreset` | `BottomEdge` | Per panel framing | Frame/bottom-edge anchor placement |
| `material_hint` | `MaterialHint` | `Glass` | Per content type | Material preset propagated to chunks |

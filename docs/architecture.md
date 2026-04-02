# Architecture

## Core Shape

`saddle-physics-destruction` follows the same split that makes NVIDIA Blast useful as a reference:

- authored fracture data is separate from runtime state
- runtime breakage is separate from physics execution
- integration happens through messages and fragment descriptors, not hard-wired rigid bodies

The crate's main data model is:

```text
FracturedAsset
  - chunks
  - bonds
  - root_chunks
  - support_chunks

Destructible entity
  - DestructionAssetHandle
  - DestructionRuntime (internal)
  - DestructionState (public)
```

## Authoring Model

Two authoring paths ship in v1:

1. `CuboidFractureBuilder`
   - deterministic jittered-grid volumetric fracture
   - stable chunk IDs
   - optional coarse parent groups
   - anchor presets for top, bottom, or none

2. `ThinSurfaceFractureBuilder`
   - deterministic 2D Voronoi-style cells extruded into depth
   - suitable for glass, boards, panels, and thin walls
   - frame/bottom-edge anchoring presets

Both builders emit a `FracturedAsset` that the runtime treats identically.

## Runtime Pipeline

The public system phases are exposed through `DestructionSystems`:

1. `AccumulateDamage`
2. `EvaluateBonds`
3. `EvaluateSupport`
4. `ActivateFragments`
5. `CleanupDebris`

Within those phases the default plugin executes:

```text
ensure_runtime_initialized
  -> process_damage_messages
  -> evaluate_accumulated_damage
  -> evaluate_support_graphs
  -> activate_pending_groups
  -> update_fragment_lifetimes
  -> cleanup_fragments
  -> sync_root_states
  -> publish_diagnostics
```

## World Space vs Asset Space

Damage messages are intentionally authored in **world space** because they usually come from gameplay hits, raycasts, explosions, or collision callbacks.

Chunk centroids and bond centers inside `FracturedAsset` are stored in **asset-local space**.

That means the runtime must convert the incoming hit into the destructible's local space before evaluating chunk or bond damage. The crate does this through the entity's `GlobalTransform`:

- hit origin: `world -> local point`
- hit direction: `world -> local vector`

Without that conversion, destructibles placed away from the world origin can appear unbreakable even though origin-based tests still pass.

## Support Graph Evaluation

Each support leaf can be anchored in one of two ways:

- authored support on the asset (`SupportKind::Fixed`, `Hanging`, `Weak`)
- per-entity override via `SupportAnchors`

When bonds change, the runtime flood-fills from anchored support leaves to find reachable chunks. Any disconnected support leaves become unsupported islands and are queued for fragment activation.

This keeps structural integrity cheap:

- no per-frame flood fill when topology is unchanged
- support recomputation only after bond or chunk detach state changes

If an asset has no anchors at all, the runtime falls back to a floating-structure rule:

- connected support components are recomputed only when topology changes
- the largest remaining component stays attached to the root
- smaller disconnected components detach as fragment groups

That keeps free-floating crates and props progressive instead of treating "no anchors" as "everything should detach immediately".

## Hierarchy And LOD

Chunks may form a simple parent/child hierarchy:

- support leaves represent fine fracture pieces
- non-support parents represent coarse activation groups

Activation LOD is driven by `DestructionViewers` and `DistanceLodBand`:

- `Full`: activate fine support leaves
- `Clustered`: prefer fewer/larger authored groups when possible
- `EventOnly`: emit detachment messages without spawning fragment entities

This keeps the core backend-neutral while still supporting near/far authoring behavior.

## Fragment Activation Contract

The crate spawns entities with:

- `Fragment`
- `FragmentLifetime`
- `FragmentSpawnData`
- inherited transform

`FragmentSpawnData` is the integration seam. It contains:

- render description
- optional collider source metadata
- initial linear/angular velocity hints
- chunk IDs represented by the fragment
- approximate size and material hint

Examples use `build_fragment_mesh` plus `StandardMaterial` to visualize fragments, but downstream code may instead spawn Avian bodies, pooled debris, custom materials, particles, or audio emitters.

When `max_chunk_spawns_per_frame` is lower than the requested fragment count, activation is queued across subsequent frames. Detachment messages still fire once per detached group, while the remaining fragment spawns drain from the backlog without being lost.

## Cleanup

Cleanup combines three mechanisms:

1. expiry through `FragmentLifetime`
2. distance trimming relative to the nearest viewer
3. global budget trimming based on `CleanupPolicy`

Budget trimming happens on the already-surviving fragments, so consumers can keep debris pressure bounded without relying only on timeouts.

## Diagnostics And Debugging

The crate publishes:

- `DestructionState` on each destructible
- `DestructionDiagnostics` as a global resource
- integration messages for start, detach, and final break

Optional gizmo output is controlled by `DestructionDebugConfig`. The debug system only runs when Bevy's gizmo resources exist, so logic-only tests and minimal apps do not need the full render/debug stack.

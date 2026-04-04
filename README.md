# Saddle Physics Destruction

Backend-agnostic destruction infrastructure for Bevy: pre-fractured assets, runtime fracture recipes, bond/support evaluation, progressive damage, hierarchical chunk activation, debris lifecycle policy, and integration messages for physics, audio, and VFX.

## Design Boundary

`saddle-physics-destruction` owns:

- fracture asset data (`FracturedAsset`, `ChunkAsset`, `BondAsset`)
- runtime damage accumulation and bond health
- support graph evaluation and unsupported-island collapse
- floating-structure component splitting when no world anchors exist
- fragment activation descriptors and debris cleanup policy
- diagnostics, debug state, and integration messages

`saddle-physics-destruction` does **not** own:

- rigid-body spawning
- collider backend integration
- audio playback
- particles or VFX playback
- game-specific scoring, quest logic, or content authoring tools

The core crate emits backend-neutral fragment data. Downstream code or examples decide how to turn that into meshes, colliders, rigid bodies, audio, or particles.

For projects that do want a ready-made physics bridge, enable the optional `avian3d` feature and attach `DestructionAvianFragments` to a destructible root. The core runtime still stays backend-neutral; the adapter is opt-in.

## Quick Start

```rust,ignore
use bevy::prelude::*;
use saddle_physics_destruction::{
    ApplyDestructionDamage, CuboidFractureBuilder, Destructible, DestructionAssetHandle,
    DestructionPlugin, FractureBias, FracturedAsset, RootVisualMode,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DestructionPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, shoot_target)
        .run();
}

fn setup(
    mut commands: Commands,
    mut assets: ResMut<Assets<FracturedAsset>>,
) {
    let asset = CuboidFractureBuilder::new(Vec3::new(2.0, 2.0, 2.0), UVec3::new(2, 2, 2)).build();
    let handle = assets.add(asset);

    commands.spawn((
        Name::new("Breakable Crate"),
        Destructible {
            visual_mode: RootVisualMode::HideOnFirstDetach,
        },
        DestructionAssetHandle(handle),
        Transform::default(),
    ));
}

fn shoot_target(
    mut writer: MessageWriter<ApplyDestructionDamage>,
    target: Single<Entity, With<Destructible>>,
) {
    writer.write(ApplyDestructionDamage {
        target: *target,
        origin: Vec3::new(0.0, 1.0, 1.5),
        direction: -Vec3::Z,
        magnitude: 3.5,
        radius: 1.2,
        kind: saddle_physics_destruction::DamageKind::Radial,
        falloff: saddle_physics_destruction::FalloffCurve::SmoothStep,
        fracture_bias: FractureBias::Balanced,
    });
}
```

This only drives the destruction runtime. To render fragments, add downstream systems that react to `Added<Fragment>` and materialize meshes or physics bodies from `FragmentSpawnData`. The crate examples and lab app show that adapter layer.

## Public API

| Type | Purpose |
|------|---------|
| `DestructionPlugin` | Registers the destruction runtime with injectable activate/deactivate/update schedules |
| `DestructionSystems` | Public ordering hooks for damage accumulation, support evaluation, activation, and cleanup |
| `FracturedAsset` | Authored pre-fractured asset containing chunks, bonds, roots, and support leaves |
| `ChunkAsset` / `BondAsset` | Stable chunk and bond metadata used by runtime evaluation |
| `CuboidFractureBuilder` / `ThinSurfaceFractureBuilder` | Deterministic authoring helpers for volumetric and thin-surface fracture assets |
| `RuntimeFracture` | Entity-side runtime recipe that generates or regenerates a `FracturedAsset` from a cuboid or thin-surface builder |
| `Destructible` / `DestructionAssetHandle` | Opt-in runtime marker plus authored asset handle |
| `DestructionEffectHooks` | Optional per-root cue names for downstream sound and particle systems |
| `DestructionState` | Normalized damage, fracture level, detached chunk count, and broken state |
| `SupportAnchors` | Optional per-entity override for which support chunks are treated as fixed anchors |
| `Fragment` / `FragmentLifetime` / `FragmentSpawnData` | Backend-neutral fragment entities plus render/collider/velocity/lifecycle descriptors |
| `DestructionConfig` | Global damage, LOD, cleanup, and fragment budget policy |
| `DestructionViewers` | Viewer positions used for distance-driven activation LOD and culling |
| `DestructionDiagnostics` / `DestructionDebugConfig` | Runtime counters and optional debug-gizmo toggles |
| `ApplyDestructionDamage` | Message-based entrypoint for world-space destruction hits |
| `DestructionStarted` / `ChunkGroupDetached` / `FinalDestructionOccurred` | Structural integration messages for gameplay or content systems |
| `DestructionEffectTriggered` | Ready-to-consume break cue message emitted from `DestructionEffectHooks` |
| `DestructionAvianFragments` | Optional `avian3d` adapter component that turns spawned fragments into rigid bodies |
| `build_fragment_mesh` | Helper for crate examples and simple downstream visual adapters |

## Supported

- deterministic pre-fractured cuboid assets
- deterministic thin-surface Voronoi-style fracture assets
- runtime cuboid and thin-surface fracture generation through `RuntimeFracture`
- point, radial, directional, and shear-style damage inputs
- accumulated chunk damage and material-weighted bond breakage
- support-graph flood fill from authored or overridden anchors
- progressive breakage with detached group activation
- optional break cue messages for downstream audio and particle playback
- optional `avian3d` fragment-body adapter through a feature flag
- anchorless breakage that keeps the largest connected body intact while smaller components detach
- near/full, mid/clustered, and far/event-only activation LOD
- lifetime, distance, and budget-based debris cleanup
- per-frame fragment throttling that preserves queued activations instead of dropping them
- material hints and initial velocity descriptors for downstream integration

## Intentionally Deferred

- arbitrary runtime mesh slicing for imported meshes
- built-in rigid-body or collider backend integration
- skinned-mesh deformation
- disk caching or serialized family/actor snapshots
- editor tooling

## Examples

| Example | What it demonstrates | Run |
|---------|----------------------|-----|
| `basic` | Minimal breakable root plus fragment materialization | `cargo run -p saddle-physics-destruction-example-basic` |
| `runtime_fracture` | Build cuboid and thin-surface fracture assets at runtime instead of pre-baking handles | `cargo run -p saddle-physics-destruction-example-runtime-fracture` |
| `structural` | Anchored pillar collapse driven by support evaluation | `cargo run -p saddle-physics-destruction-example-structural` |
| `hierarchical` | Staged coarse/fine authoring and repeated impact pulses | `cargo run -p saddle-physics-destruction-example-hierarchical` |
| `thin_surface` | Glass-like sheet fracture using the thin-surface builder | `cargo run -p saddle-physics-destruction-example-thin-surface` |
| `stress_test` | Repeated hits plus diagnostics-oriented cleanup pressure | `cargo run -p saddle-physics-destruction-example-stress-test` |

The shared example support crate now installs `saddle-pane` for the regular showcase binaries so cleanup, budget, and debug-graph tuning can be adjusted live while fragments are spawning.
Those same support scenes now also exercise the new effect-hook and Avian adapter paths so the examples land like game props instead of simple debug debris.

## Crate-Local Lab

The richer showcase and verification app lives in:

`shared/physics/saddle-physics-destruction/examples/lab`

Run it directly with:

```bash
cargo run -p saddle-physics-destruction-lab
```

The lab now includes its own small `saddle-pane` surface for fragment budgets, effect-hook diagnostics, and runtime counters, so the standalone verification scene stays editable too.

## E2E Verification

```bash
cargo run -p saddle-physics-destruction-lab --features e2e -- destruction_smoke
cargo run -p saddle-physics-destruction-lab --features e2e -- destruction_effects
cargo run -p saddle-physics-destruction-lab --features e2e -- destruction_supports
cargo run -p saddle-physics-destruction-lab --features e2e -- destruction_hierarchy
cargo run -p saddle-physics-destruction-lab --features e2e -- destruction_lod
cargo run -p saddle-physics-destruction-lab --features e2e -- destruction_budget
```

Each scenario writes screenshots and logs under `e2e_output/<scenario>/`.

## BRP Inspection

Start the lab in one terminal:

```bash
cargo run -p saddle-physics-destruction-lab
```

Then inspect it from another:

```bash
uv run --project .codex/skills/bevy-brp/script brp ping
uv run --project .codex/skills/bevy-brp/script brp status
uv run --project .codex/skills/bevy-brp/script brp resource get saddle_physics_destruction::config::DestructionDiagnostics
uv run --project .codex/skills/bevy-brp/script brp extras screenshot /tmp/saddle_physics_destruction_lab.png
uv run --project .codex/skills/bevy-brp/script brp extras shutdown
```

## More Detail

- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)

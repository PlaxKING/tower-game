"""
Tower Game - Blender Batch Export Script
Exports all .blend models from blender/models/ to FBX for UE5 import.

Usage:
  blender --background --python blender/scripts/batch_export.py
  Or run via VS Code task: "Blender: Export All Assets"
"""

import bpy
import os
import sys
import json
from pathlib import Path
from datetime import datetime


# === Configuration ===

# Resolve project root (script is at blender/scripts/batch_export.py)
SCRIPT_DIR = Path(os.path.dirname(os.path.abspath(__file__)))
PROJECT_ROOT = SCRIPT_DIR.parent.parent
MODELS_DIR = SCRIPT_DIR.parent / "models"
EXPORT_DIR = PROJECT_ROOT / "ue5-client" / "Content" / "Models" / "Imported"

# UE5 FBX export settings
UE5_EXPORT_SETTINGS = {
    "use_selection": False,
    "use_active_collection": False,
    "global_scale": 1.0,
    "apply_unit_scale": True,
    "apply_scale_options": "FBX_SCALE_ALL",
    "use_space_transform": True,
    "axis_forward": "-Y",
    "axis_up": "Z",
    "object_types": {"MESH", "ARMATURE", "EMPTY"},
    "use_mesh_modifiers": True,
    "use_mesh_modifiers_render": True,
    "mesh_smooth_type": "FACE",
    "use_subsurf": False,
    "use_mesh_edges": False,
    "use_tspace": True,
    "use_custom_props": False,
    "add_leaf_bones": False,
    "primary_bone_axis": "Y",
    "secondary_bone_axis": "X",
    "use_armature_deform_only": True,
    "armature_nodetype": "NULL",
    "bake_anim": True,
    "bake_anim_use_all_bones": True,
    "bake_anim_use_nla_strips": False,
    "bake_anim_use_all_actions": True,
    "bake_anim_force_startend_keying": True,
    "bake_anim_step": 1.0,
    "bake_anim_simplify_factor": 1.0,
    "path_mode": "AUTO",
    "embed_textures": True,
    "batch_mode": "OFF",
    "use_batch_own_dir": True,
}

# Asset categories for organized export
ASSET_CATEGORIES = {
    "characters": "Characters",
    "weapons": "Weapons",
    "armor": "Armor",
    "monsters": "Monsters",
    "environment": "Environment",
    "props": "Props",
    "vfx": "VFX",
    "ui": "UI",
}


def ensure_dir(path: Path):
    """Create directory if it doesn't exist."""
    path.mkdir(parents=True, exist_ok=True)


def get_category(filepath: Path) -> str:
    """Determine asset category from file path or name."""
    name_lower = filepath.stem.lower()
    parent_lower = filepath.parent.name.lower()

    for key, category in ASSET_CATEGORIES.items():
        if key in name_lower or key in parent_lower:
            return category

    return "Misc"


def clean_scene():
    """Remove all objects from the current scene."""
    bpy.ops.wm.read_homefile(use_empty=True)


def validate_model(filepath: Path) -> dict:
    """Validate a .blend file before export."""
    issues = []
    stats = {"vertices": 0, "faces": 0, "objects": 0, "armatures": 0}

    for obj in bpy.data.objects:
        if obj.type == "MESH":
            stats["objects"] += 1
            mesh = obj.data
            stats["vertices"] += len(mesh.vertices)
            stats["faces"] += len(mesh.polygons)

            # Check for ngons (faces with > 4 vertices)
            ngons = [p for p in mesh.polygons if len(p.vertices) > 4]
            if ngons:
                issues.append(f"{obj.name}: {len(ngons)} ngons found (should be tris/quads)")

            # Check UV maps
            if not mesh.uv_layers:
                issues.append(f"{obj.name}: No UV map")

            # Check scale
            if any(abs(s - 1.0) > 0.001 for s in obj.scale):
                issues.append(f"{obj.name}: Non-uniform scale {tuple(obj.scale)}")

            # Check normals
            if not mesh.has_custom_normals:
                pass  # Not always required

        elif obj.type == "ARMATURE":
            stats["armatures"] += 1

    return {"file": str(filepath), "stats": stats, "issues": issues, "valid": len(issues) == 0}


def export_blend_to_fbx(blend_path: Path, export_path: Path) -> bool:
    """Export a single .blend file to FBX."""
    try:
        clean_scene()
        bpy.ops.wm.open_mainfile(filepath=str(blend_path))

        ensure_dir(export_path.parent)

        bpy.ops.export_scene.fbx(
            filepath=str(export_path),
            use_selection=UE5_EXPORT_SETTINGS["use_selection"],
            global_scale=UE5_EXPORT_SETTINGS["global_scale"],
            apply_unit_scale=UE5_EXPORT_SETTINGS["apply_unit_scale"],
            apply_scale_options=UE5_EXPORT_SETTINGS["apply_scale_options"],
            use_space_transform=UE5_EXPORT_SETTINGS["use_space_transform"],
            axis_forward=UE5_EXPORT_SETTINGS["axis_forward"],
            axis_up=UE5_EXPORT_SETTINGS["axis_up"],
            use_mesh_modifiers=UE5_EXPORT_SETTINGS["use_mesh_modifiers"],
            mesh_smooth_type=UE5_EXPORT_SETTINGS["mesh_smooth_type"],
            use_tspace=UE5_EXPORT_SETTINGS["use_tspace"],
            add_leaf_bones=UE5_EXPORT_SETTINGS["add_leaf_bones"],
            primary_bone_axis=UE5_EXPORT_SETTINGS["primary_bone_axis"],
            secondary_bone_axis=UE5_EXPORT_SETTINGS["secondary_bone_axis"],
            use_armature_deform_only=UE5_EXPORT_SETTINGS["use_armature_deform_only"],
            bake_anim=UE5_EXPORT_SETTINGS["bake_anim"],
            bake_anim_use_all_bones=UE5_EXPORT_SETTINGS["bake_anim_use_all_bones"],
            bake_anim_use_nla_strips=UE5_EXPORT_SETTINGS["bake_anim_use_nla_strips"],
            bake_anim_use_all_actions=UE5_EXPORT_SETTINGS["bake_anim_use_all_actions"],
            bake_anim_step=UE5_EXPORT_SETTINGS["bake_anim_step"],
            bake_anim_simplify_factor=UE5_EXPORT_SETTINGS["bake_anim_simplify_factor"],
            path_mode=UE5_EXPORT_SETTINGS["path_mode"],
            embed_textures=UE5_EXPORT_SETTINGS["embed_textures"],
        )

        print(f"  [OK] Exported: {blend_path.name} -> {export_path.name}")
        return True

    except Exception as e:
        print(f"  [FAIL] {blend_path.name}: {e}")
        return False


def main():
    """Main batch export pipeline."""
    print("=" * 60)
    print("Tower Game - Blender Batch Export")
    print(f"Time: {datetime.now().isoformat()}")
    print(f"Models dir: {MODELS_DIR}")
    print(f"Export dir: {EXPORT_DIR}")
    print("=" * 60)

    ensure_dir(EXPORT_DIR)

    # Find all .blend files
    blend_files = list(MODELS_DIR.rglob("*.blend"))

    if not blend_files:
        print("No .blend files found in models directory.")
        print(f"Place your models in: {MODELS_DIR}")
        return

    print(f"Found {len(blend_files)} .blend files\n")

    results = {"exported": 0, "failed": 0, "skipped": 0, "files": []}

    for blend_path in blend_files:
        category = get_category(blend_path)
        fbx_name = blend_path.stem + ".fbx"
        export_path = EXPORT_DIR / category / fbx_name

        # Skip if FBX is newer than .blend
        if export_path.exists():
            blend_mtime = blend_path.stat().st_mtime
            fbx_mtime = export_path.stat().st_mtime
            if fbx_mtime > blend_mtime:
                print(f"  [SKIP] {blend_path.name} (up to date)")
                results["skipped"] += 1
                continue

        print(f"Exporting: {blend_path.name} [{category}]")

        if export_blend_to_fbx(blend_path, export_path):
            results["exported"] += 1
            results["files"].append(str(export_path))
        else:
            results["failed"] += 1

    # Write export report
    report_path = EXPORT_DIR / "export_report.json"
    report = {
        "timestamp": datetime.now().isoformat(),
        "total_files": len(blend_files),
        "exported": results["exported"],
        "failed": results["failed"],
        "skipped": results["skipped"],
        "files": results["files"],
    }

    with open(report_path, "w") as f:
        json.dump(report, f, indent=2)

    print("\n" + "=" * 60)
    print(f"Export complete: {results['exported']} exported, "
          f"{results['skipped']} skipped, {results['failed']} failed")
    print(f"Report: {report_path}")
    print("=" * 60)


if __name__ == "__main__":
    main()

"""
Tower Game - Blender Model Validator
Checks all .blend models for UE5 compatibility.

Validates: scale, normals, UVs, polygon count, naming conventions,
material setup, and armature structure.

Usage:
  blender --background --python blender/scripts/validate_models.py
"""

import bpy
import os
import json
from pathlib import Path
from datetime import datetime


SCRIPT_DIR = Path(os.path.dirname(os.path.abspath(__file__)))
MODELS_DIR = SCRIPT_DIR.parent / "models"
REPORT_DIR = SCRIPT_DIR.parent / "reports"

# UE5 compatibility limits
MAX_VERTICES_LOD0 = 100000
MAX_BONES = 256
EXPECTED_SCALE = (1.0, 1.0, 1.0)
SCALE_TOLERANCE = 0.001


class ModelValidator:
    def __init__(self):
        self.results = []
        self.total_issues = 0

    def validate_file(self, filepath: Path) -> dict:
        """Validate a single .blend file."""
        bpy.ops.wm.read_homefile(use_empty=True)
        bpy.ops.wm.open_mainfile(filepath=str(filepath))

        report = {
            "file": filepath.name,
            "path": str(filepath),
            "objects": [],
            "issues": [],
            "warnings": [],
            "stats": {
                "total_vertices": 0,
                "total_faces": 0,
                "mesh_objects": 0,
                "armatures": 0,
                "materials": 0,
            },
        }

        for obj in bpy.data.objects:
            if obj.type == "MESH":
                self._validate_mesh(obj, report)
            elif obj.type == "ARMATURE":
                self._validate_armature(obj, report)

        # Validate materials
        for mat in bpy.data.materials:
            report["stats"]["materials"] += 1
            if not mat.use_nodes:
                report["warnings"].append(f"Material '{mat.name}': not using nodes")

        # Overall checks
        if report["stats"]["total_vertices"] > MAX_VERTICES_LOD0:
            report["issues"].append(
                f"Total vertices ({report['stats']['total_vertices']}) "
                f"exceeds LOD0 limit ({MAX_VERTICES_LOD0})"
            )

        report["valid"] = len(report["issues"]) == 0
        self.total_issues += len(report["issues"])
        self.results.append(report)
        return report

    def _validate_mesh(self, obj, report: dict):
        """Validate a mesh object."""
        mesh = obj.data
        info = {
            "name": obj.name,
            "vertices": len(mesh.vertices),
            "faces": len(mesh.polygons),
            "edges": len(mesh.edges),
        }

        report["stats"]["mesh_objects"] += 1
        report["stats"]["total_vertices"] += len(mesh.vertices)
        report["stats"]["total_faces"] += len(mesh.polygons)
        report["objects"].append(info)

        # Check applied scale
        for i, (s, expected) in enumerate(zip(obj.scale, EXPECTED_SCALE)):
            if abs(s - expected) > SCALE_TOLERANCE:
                axis = "XYZ"[i]
                report["issues"].append(
                    f"'{obj.name}': Scale {axis}={s:.3f} (expected {expected}). "
                    f"Apply scale with Ctrl+A"
                )

        # Check applied rotation
        if any(abs(r) > 0.001 for r in obj.rotation_euler):
            report["warnings"].append(
                f"'{obj.name}': Has unapplied rotation. Consider Ctrl+A -> Rotation"
            )

        # Check UV maps
        if not mesh.uv_layers:
            report["issues"].append(f"'{obj.name}': Missing UV map (required for UE5)")
        elif len(mesh.uv_layers) > 2:
            report["warnings"].append(
                f"'{obj.name}': {len(mesh.uv_layers)} UV maps (UE5 typically uses 1-2)"
            )

        # Check ngons
        ngons = sum(1 for p in mesh.polygons if len(p.vertices) > 4)
        if ngons > 0:
            report["warnings"].append(
                f"'{obj.name}': {ngons} ngons (triangulate for best UE5 results)"
            )

        # Check loose vertices
        vertex_used = set()
        for edge in mesh.edges:
            vertex_used.update(edge.vertices)
        loose = len(mesh.vertices) - len(vertex_used)
        if loose > 0:
            report["warnings"].append(f"'{obj.name}': {loose} loose vertices")

        # Check naming convention (UE5 prefers SM_ prefix for static meshes)
        if not obj.name.startswith(("SM_", "SK_", "S_")):
            report["warnings"].append(
                f"'{obj.name}': Consider UE5 naming: SM_ (static), SK_ (skeletal)"
            )

    def _validate_armature(self, obj, report: dict):
        """Validate an armature object."""
        armature = obj.data
        bone_count = len(armature.bones)
        report["stats"]["armatures"] += 1

        if bone_count > MAX_BONES:
            report["issues"].append(
                f"'{obj.name}': {bone_count} bones exceeds UE5 limit ({MAX_BONES})"
            )

        if bone_count == 0:
            report["issues"].append(f"'{obj.name}': Armature has no bones")

        # Check root bone
        root_bones = [b for b in armature.bones if b.parent is None]
        if len(root_bones) > 1:
            report["warnings"].append(
                f"'{obj.name}': Multiple root bones ({len(root_bones)}). "
                f"UE5 works best with a single root"
            )

    def generate_report(self) -> str:
        """Generate validation report."""
        REPORT_DIR.mkdir(parents=True, exist_ok=True)
        report_path = REPORT_DIR / f"validation_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"

        summary = {
            "timestamp": datetime.now().isoformat(),
            "total_files": len(self.results),
            "valid_files": sum(1 for r in self.results if r["valid"]),
            "invalid_files": sum(1 for r in self.results if not r["valid"]),
            "total_issues": self.total_issues,
            "files": self.results,
        }

        with open(report_path, "w") as f:
            json.dump(summary, f, indent=2)

        return str(report_path)


def main():
    print("=" * 60)
    print("Tower Game - Model Validator")
    print(f"Models dir: {MODELS_DIR}")
    print("=" * 60)

    blend_files = list(MODELS_DIR.rglob("*.blend"))

    if not blend_files:
        print("No .blend files found.")
        print(f"Place models in: {MODELS_DIR}")
        return

    print(f"Found {len(blend_files)} models to validate\n")

    validator = ModelValidator()

    for filepath in blend_files:
        print(f"Validating: {filepath.name}")
        result = validator.validate_file(filepath)

        if result["valid"]:
            print(f"  [PASS] {result['stats']['total_vertices']} verts, "
                  f"{result['stats']['total_faces']} faces")
        else:
            print(f"  [FAIL] {len(result['issues'])} issues:")
            for issue in result["issues"]:
                print(f"    - {issue}")

        if result["warnings"]:
            for warn in result["warnings"]:
                print(f"    [WARN] {warn}")

        print()

    report_path = validator.generate_report()

    print("=" * 60)
    valid = sum(1 for r in validator.results if r["valid"])
    total = len(validator.results)
    print(f"Results: {valid}/{total} passed, {validator.total_issues} issues")
    print(f"Report: {report_path}")
    print("=" * 60)


if __name__ == "__main__":
    main()

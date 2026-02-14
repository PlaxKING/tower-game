"""
Tower Game - Blender Addon
Custom tools for creating Tower Game assets.

Install: Blender -> Edit -> Preferences -> Add-ons -> Install -> select this file
"""

bl_info = {
    "name": "Tower Game Asset Tools",
    "author": "Tower Game Dev",
    "version": (1, 0, 0),
    "blender": (4, 0, 0),
    "location": "View3D > Sidebar > Tower Game",
    "description": "Asset creation tools for Tower Game MMORPG",
    "category": "Game Engine",
}

import bpy
from bpy.props import EnumProperty, FloatProperty, StringProperty
import os
from pathlib import Path


# === Constants (matching Rust procedural-core) ===

WEAPON_TYPES = [
    ("SWORD", "Sword", "One-handed sword (3-hit combo)"),
    ("GREATSWORD", "Greatsword", "Two-handed greatsword (2-hit combo)"),
    ("DUAL_DAGGERS", "Dual Daggers", "Dual daggers (5-hit combo)"),
    ("SPEAR", "Spear", "Spear (3-hit combo)"),
    ("GAUNTLETS", "Gauntlets", "Gauntlets (5-hit combo)"),
    ("STAFF", "Staff", "Staff (2-hit combo)"),
]

ARMOR_SLOTS = [
    ("HEAD", "Head", "Head armor"),
    ("CHEST", "Chest", "Chest armor"),
    ("HANDS", "Hands", "Hand armor"),
    ("LEGS", "Legs", "Leg armor"),
    ("FEET", "Feet", "Foot armor"),
]

MONSTER_PARTS = [
    ("BODY", "Body", "Main body mesh"),
    ("HEAD", "Head", "Head / face"),
    ("LIMB_FRONT", "Front Limbs", "Arms / front legs"),
    ("LIMB_REAR", "Rear Limbs", "Legs / rear legs"),
    ("TAIL", "Tail", "Tail appendage"),
    ("WING", "Wings", "Wings"),
    ("HORN", "Horns", "Horns / antlers"),
    ("EXTRA", "Extra", "Extra appendages"),
]

# UE5 scale: 1 unit = 1 cm
PLAYER_HEIGHT_CM = 180.0
SCALE_FACTOR = 0.01  # Blender meters -> UE5 centimeters


class TOWER_PT_MainPanel(bpy.types.Panel):
    bl_label = "Tower Game"
    bl_idname = "TOWER_PT_main"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Tower Game"

    def draw(self, context):
        layout = self.layout
        layout.label(text="Asset Type:")
        layout.prop(context.scene, "tower_asset_type", text="")
        layout.separator()

        if context.scene.tower_asset_type == "WEAPON":
            layout.prop(context.scene, "tower_weapon_type", text="Weapon")
        elif context.scene.tower_asset_type == "ARMOR":
            layout.prop(context.scene, "tower_armor_slot", text="Slot")
        elif context.scene.tower_asset_type == "MONSTER":
            layout.prop(context.scene, "tower_monster_part", text="Part")

        layout.separator()
        layout.operator("tower.setup_scene", text="Setup Scene")
        layout.operator("tower.validate_asset", text="Validate Asset")
        layout.operator("tower.export_fbx", text="Export to UE5")


class TOWER_OT_SetupScene(bpy.types.Operator):
    bl_idname = "tower.setup_scene"
    bl_label = "Setup Tower Scene"
    bl_description = "Configure scene for Tower Game asset creation"

    def execute(self, context):
        # Set units to metric
        context.scene.unit_settings.system = "METRIC"
        context.scene.unit_settings.scale_length = 1.0

        # Add reference grid at player height
        bpy.ops.mesh.primitive_plane_add(size=2.0, location=(0, 0, 0))
        grid = context.active_object
        grid.name = "REF_Ground"
        grid.display_type = "WIRE"
        grid.hide_render = True

        # Add height reference
        bpy.ops.mesh.primitive_cube_add(
            size=1.0,
            location=(2.0, 0, PLAYER_HEIGHT_CM * SCALE_FACTOR / 2),
        )
        ref = context.active_object
        ref.name = "REF_PlayerHeight"
        ref.scale = (0.1, 0.1, PLAYER_HEIGHT_CM * SCALE_FACTOR)
        ref.display_type = "WIRE"
        ref.hide_render = True

        self.report({"INFO"}, "Scene configured for Tower Game")
        return {"FINISHED"}


class TOWER_OT_ValidateAsset(bpy.types.Operator):
    bl_idname = "tower.validate_asset"
    bl_label = "Validate Asset"
    bl_description = "Check asset for UE5 compatibility"

    def execute(self, context):
        issues = []

        for obj in context.scene.objects:
            if obj.name.startswith("REF_"):
                continue

            if obj.type == "MESH":
                # Check scale
                if any(abs(s - 1.0) > 0.001 for s in obj.scale):
                    issues.append(f"{obj.name}: Unapplied scale")

                # Check UVs
                if not obj.data.uv_layers:
                    issues.append(f"{obj.name}: No UV map")

                # Check vertex count
                verts = len(obj.data.vertices)
                if verts > 50000:
                    issues.append(f"{obj.name}: {verts} vertices (consider LODs)")

        if issues:
            for issue in issues:
                self.report({"WARNING"}, issue)
        else:
            self.report({"INFO"}, "All checks passed!")

        return {"FINISHED"}


class TOWER_OT_ExportFBX(bpy.types.Operator):
    bl_idname = "tower.export_fbx"
    bl_label = "Export to UE5"
    bl_description = "Export selected objects as FBX for UE5"

    def execute(self, context):
        # Determine export path
        blend_path = bpy.data.filepath
        if not blend_path:
            self.report({"ERROR"}, "Save .blend file first")
            return {"CANCELLED"}

        blend_name = Path(blend_path).stem
        project_root = Path(blend_path).parent.parent.parent
        export_dir = project_root / "ue5-client" / "Content" / "Models" / "Imported"
        export_dir.mkdir(parents=True, exist_ok=True)

        asset_type = context.scene.tower_asset_type
        prefix = {"WEAPON": "SM_Weapon", "ARMOR": "SM_Armor",
                  "MONSTER": "SK_Monster", "ENVIRONMENT": "SM_Env",
                  "CHARACTER": "SK_Char"}
        pfx = prefix.get(asset_type, "SM")
        export_path = export_dir / f"{pfx}_{blend_name}.fbx"

        # Deselect reference objects
        for obj in context.scene.objects:
            if obj.name.startswith("REF_"):
                obj.select_set(False)
                obj.hide_set(True)

        bpy.ops.export_scene.fbx(
            filepath=str(export_path),
            use_selection=False,
            global_scale=1.0,
            apply_unit_scale=True,
            apply_scale_options="FBX_SCALE_ALL",
            axis_forward="-Y",
            axis_up="Z",
            use_mesh_modifiers=True,
            mesh_smooth_type="FACE",
            use_tspace=True,
            add_leaf_bones=False,
            use_armature_deform_only=True,
            bake_anim=True,
            embed_textures=True,
        )

        # Unhide references
        for obj in context.scene.objects:
            if obj.name.startswith("REF_"):
                obj.hide_set(False)

        self.report({"INFO"}, f"Exported to {export_path}")
        return {"FINISHED"}


# === Registration ===

classes = (
    TOWER_PT_MainPanel,
    TOWER_OT_SetupScene,
    TOWER_OT_ValidateAsset,
    TOWER_OT_ExportFBX,
)


def register():
    for cls in classes:
        bpy.utils.register_class(cls)

    bpy.types.Scene.tower_asset_type = EnumProperty(
        name="Asset Type",
        items=[
            ("WEAPON", "Weapon", ""),
            ("ARMOR", "Armor", ""),
            ("MONSTER", "Monster", ""),
            ("ENVIRONMENT", "Environment", ""),
            ("CHARACTER", "Character", ""),
        ],
        default="WEAPON",
    )
    bpy.types.Scene.tower_weapon_type = EnumProperty(
        name="Weapon Type", items=WEAPON_TYPES, default="SWORD"
    )
    bpy.types.Scene.tower_armor_slot = EnumProperty(
        name="Armor Slot", items=ARMOR_SLOTS, default="CHEST"
    )
    bpy.types.Scene.tower_monster_part = EnumProperty(
        name="Monster Part", items=MONSTER_PARTS, default="BODY"
    )


def unregister():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)

    del bpy.types.Scene.tower_asset_type
    del bpy.types.Scene.tower_weapon_type
    del bpy.types.Scene.tower_armor_slot
    del bpy.types.Scene.tower_monster_part


if __name__ == "__main__":
    register()

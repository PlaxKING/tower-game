// Copyright Tower Game. All Rights Reserved.

using UnrealBuildTool;
using System.Collections.Generic;

public class TowerGameEditorTarget : TargetRules
{
	public TowerGameEditorTarget(TargetInfo Target) : base(Target)
	{
		Type = TargetType.Editor;
		DefaultBuildSettings = BuildSettingsVersion.V4;
		IncludeOrderVersion = EngineIncludeOrderVersion.Unreal5_3;

		ExtraModuleNames.Add("TowerGame");
	}
}

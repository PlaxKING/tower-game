// Copyright Tower Game. All Rights Reserved.

using UnrealBuildTool;
using System.Collections.Generic;

public class TowerGameTarget : TargetRules
{
	public TowerGameTarget(TargetInfo Target) : base(Target)
	{
		Type = TargetType.Game;
		DefaultBuildSettings = BuildSettingsVersion.V4;
		IncludeOrderVersion = EngineIncludeOrderVersion.Unreal5_3;

		ExtraModuleNames.Add("TowerGame");
	}
}

using UnrealBuildTool;

public class TowerGame : ModuleRules
{
    public TowerGame(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

        PublicDependencyModuleNames.AddRange(new string[] {
            "Core",
            "CoreUObject",
            "Engine",
            "InputCore",
            "EnhancedInput",
            "Niagara",
            "Json",
            "JsonUtilities",
            "UMG",
            "NetCore",
            "HTTP",
            "Slate",
            "SlateCore",
            "WebSockets"
        });

        PrivateDependencyModuleNames.AddRange(new string[] {
            "Projects"
        });

        // Include paths for our module subdirectories
        PublicIncludePaths.AddRange(new string[] {
            "TowerGame"
        });
    }
}

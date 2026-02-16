using UnrealBuildTool;
using System.IO;

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

        // ============================================================
        // Rust Procedural Core DLL Integration (tower_core.dll v0.6.0)
        // ============================================================

        // Path to the Rust DLL (relative to project root)
        string ProjectRoot = Path.GetFullPath(Path.Combine(ModuleDirectory, "../../.."));
        string RustDllPath = Path.Combine(ProjectRoot, "../procedural-core/target/release/tower_core.dll");
        string BinariesDir = Path.Combine(ModuleDirectory, "../../Binaries/Win64");
        string TargetDllPath = Path.Combine(BinariesDir, "tower_core.dll");

        // Ensure Binaries directory exists
        if (!Directory.Exists(BinariesDir))
        {
            Directory.CreateDirectory(BinariesDir);
        }

        // Copy DLL to Binaries folder if source exists
        if (File.Exists(RustDllPath))
        {
            if (!File.Exists(TargetDllPath) ||
                File.GetLastWriteTime(RustDllPath) > File.GetLastWriteTime(TargetDllPath))
            {
                File.Copy(RustDllPath, TargetDllPath, true);
            }
        }

        // Tell UE5 to delay-load the DLL
        PublicDelayLoadDLLs.Add("tower_core.dll");

        // Add as runtime dependency so it gets packaged
        RuntimeDependencies.Add(TargetDllPath);
    }
}

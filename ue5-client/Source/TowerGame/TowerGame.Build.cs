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
            "WebSockets",
            "NavigationSystem"
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

        // Path to the Rust DLL (MSVC build for proper .lib generation)
        string ProjectRoot = Path.GetFullPath(Path.Combine(ModuleDirectory, "../../.."));
        string RustBuildDir = Path.Combine(ProjectRoot, "../procedural-core/target/x86_64-pc-windows-msvc/release");
        string RustDllPath = Path.Combine(RustBuildDir, "tower_core.dll");
        string RustLibPath = Path.Combine(RustBuildDir, "tower_core.dll.lib");

        string PluginBinDir = Path.Combine(ModuleDirectory, "../../Plugins/ProceduralCore/Binaries/Win64");
        string TargetDllPath = Path.Combine(PluginBinDir, "tower_core.dll");
        string TargetLibPath = Path.Combine(PluginBinDir, "tower_core.dll.lib");

        // Ensure Plugin Binaries directory exists
        if (!Directory.Exists(PluginBinDir))
        {
            Directory.CreateDirectory(PluginBinDir);
        }

        // Copy DLL to Plugin Binaries folder if source exists
        if (File.Exists(RustDllPath))
        {
            if (!File.Exists(TargetDllPath) ||
                File.GetLastWriteTime(RustDllPath) > File.GetLastWriteTime(TargetDllPath))
            {
                File.Copy(RustDllPath, TargetDllPath, true);
            }
        }

        // Copy .lib to Plugin Binaries folder for linking
        if (File.Exists(RustLibPath))
        {
            if (!File.Exists(TargetLibPath) ||
                File.GetLastWriteTime(RustLibPath) > File.GetLastWriteTime(TargetLibPath))
            {
                File.Copy(RustLibPath, TargetLibPath, true);
            }
        }

        // Tell linker where to find the import library
        PublicAdditionalLibraries.Add(TargetLibPath);

        // Tell UE5 to delay-load the DLL
        PublicDelayLoadDLLs.Add("tower_core.dll");

        // Add as runtime dependency so it gets packaged
        RuntimeDependencies.Add(TargetDllPath);
    }
}

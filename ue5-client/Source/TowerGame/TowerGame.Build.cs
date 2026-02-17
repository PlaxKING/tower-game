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
            "Sockets",          // UDP socket support
            "Networking",       // Network utilities
            "Slate",
            "SlateCore",
            "WebSockets",
            "NavigationSystem",
            "GeometryCollectionEngine",  // Chaos Destruction system
            "FieldSystemEngine",         // Field system for destruction forces
            "ChaosSolverEngine"          // Chaos physics solver
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

        // ============================================================================
        // Tower Bevy Server DLL (FFI Integration for Protobuf)
        // ============================================================================

        string BevyThirdParty = Path.Combine(ModuleDirectory, "../../ThirdParty/TowerBevy");
        string BevyInclude = Path.Combine(BevyThirdParty, "include");
        string BevyLib = Path.Combine(BevyThirdParty, "lib");
        string BevyDllPath = Path.Combine(BevyLib, "tower_bevy_server.dll");

        // Add include path for FFI header
        PublicIncludePaths.Add(BevyInclude);

        // Link with DLL (delay-load for graceful fallback)
        if (File.Exists(BevyDllPath))
        {
            PublicDelayLoadDLLs.Add("tower_bevy_server.dll");
            RuntimeDependencies.Add(BevyDllPath);
        }

        // ============================================================================
        // NOTE: Using Rust FFI for Protobuf instead of libprotobuf.lib
        // ============================================================================
        // tower_bevy_server.dll provides C FFI functions:
        //   - protobuf_to_json()  (Protobuf binary → JSON string)
        //   - free_string()       (Free Rust-allocated strings)
        //   - get_chunk_field()   (Extract specific fields)
        //
        // Advantages:
        //   - No need for libprotobuf.lib in UE5
        //   - Rust handles all Protobuf parsing (prost crate)
        //   - Simpler build process
        //   - 2.5MB DLL includes everything needed
    }
}

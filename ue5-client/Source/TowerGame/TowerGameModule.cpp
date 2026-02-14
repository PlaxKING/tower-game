#include "TowerGameModule.h"
#include "Modules/ModuleManager.h"

void FTowerGameModule::StartupModule()
{
    UE_LOG(LogTemp, Log, TEXT("TowerGame module starting up"));
}

void FTowerGameModule::ShutdownModule()
{
    UE_LOG(LogTemp, Log, TEXT("TowerGame module shutting down"));
}

IMPLEMENT_PRIMARY_GAME_MODULE(FTowerGameModule, TowerGame, "TowerGame");

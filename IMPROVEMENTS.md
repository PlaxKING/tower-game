# Tower Game - Improvement Proposals & Technical Debt
# Предложения по улучшению проекта и технический долг

> **Правило**: Обновляй этот файл после каждой сессии.
> Приоритеты: P0 (критично) → P1 (высокий) → P2 (средний) → P3 (низкий)

---

## Предложения по улучшению (IMP-001 — IMP-XXX)

### IMP-001: Перейти с JSON-over-HTTP на настоящий gRPC
- **Приоритет**: P1
- **Статус**: Предложено
- **Описание**: Сейчас Rust↔UE5 общаются через JSON-over-HTTP. gRPC (Protocol Buffers) даст:
  - Типобезопасность на обоих концах
  - ~3x меньше сетевого трафика (бинарный формат)
  - Стриминг (ServerStream для обновлений мира)
  - Автогенерация клиента для UE5
- **Необходимо**:
  - `tonic` crate в Rust для gRPC сервера
  - `grpc` plugin для UE5 (или custom HTTP/2 клиент)
  - Компиляция .proto файлов через `build.rs`
- **Файлы**: shared/proto/services.proto уже готов
- **Оценка сложности**: Средняя (proto файлы есть, нужна интеграция)

### IMP-002: Добавить build.rs для автогенерации Proto
- **Приоритет**: P1
- **Статус**: Предложено
- **Описание**: Создать `build.rs` в procedural-core для автоматической компиляции .proto файлов при `cargo build`
- **Необходимо**:
  - `prost-build` + `tonic-build` в build-dependencies
  - `build.rs` с путями к .proto файлам
  - `include!(concat!(env!("OUT_DIR"), "/..."))` в модулях
- **Выгода**: Типобезопасная сериализация вместо ручного JSON

### IMP-003: Интеграционные тесты Rust↔JSON
- **Приоритет**: P1
- **Статус**: Выполнено (Сессия 16)
- **Описание**: Создать тесты, которые проверяют что JSON из Rust корректно парсится обратно и совпадает с ожидаемым UE5 форматом
- **Необходимо**:
  - Файл `tests/integration_json.rs`
  - Тестовые JSON fixtures (`tests/fixtures/*.json`)
  - Проверка round-trip: Struct → JSON → Struct
- **Выгода**: Предотвращение CE-300 (JSON format mismatch)

### IMP-004: Property-based testing с proptest
- **Приоритет**: P2
- **Статус**: Выполнено (Сессия 17)
- **Описание**: 15 proptest тестов по 8 подсистемам (Floor, Monster, Combat, Mastery, Season, Socket, Achievement, Loot, Breath)
- **Файл**: `tests/property_tests.rs`
- **Выгода**: Находит edge cases, которые ручные тесты пропускают

### IMP-005: Бенчмарки для критических путей
- **Приоритет**: P2
- **Статус**: Выполнено (Сессия 16)
- **Описание**: criterion бенчмарки для:
  - Генерация этажа, layout, hash
  - Монстры (одиночный + batch)
  - Комбат (calculate_combat, angle)
  - Семантическая похожесть
  - Лут, mastery, abilities, social
  - Full round-trip (mastery, ability)
- **Файл**: `benches/generation_bench.rs` (14 benchmarks)
- **Выгода**: Отслеживание производительности между сессиями

### IMP-006: Расширить FFI bridge для новых модулей
- **Приоритет**: P1
- **Статус**: Выполнено (Сессия 15-16)
- **Описание**: bridge/mod.rs расширен с 16 до 46 C-ABI функций, покрывает все системы:
  - Mastery (5), Specialization (4), Abilities (4), Sockets (6)
  - Cosmetics (6), Tutorial (5), Achievements (4), Seasons (5), Social (9)
- **UE5 Bridge**: ProceduralCoreBridge.h/.cpp обновлён для 46 exports

### IMP-007: Error handling — Result вместо unwrap
- **Приоритет**: P2
- **Статус**: Выполнено (Сессия 17 — подтверждено, что уже готово)
- **Описание**: Аудит показал: все 46 production FFI функций уже используют safe error handling (match/return null, unwrap_or_default). unwrap() только в #[cfg(test)] коде.
- **Выгода**: Стабильность DLL, предсказуемые ошибки

### IMP-008: Документация API для UE5 разработчиков
- **Приоритет**: P3
- **Статус**: ✅ Завершено (Session 22)
- **Описание**: Создать чёткую документацию формата JSON для каждого FFI вызова
- **Реализация**: `docs/api-reference.md` — 100 FFI функций, 23 категории
- **Содержание**: Для каждого FFI export: входные параметры, JSON формат ответа, коды ошибок, примеры
  - Memory management guidelines
  - Error handling conventions (null checks)
  - Performance notes (caching, batching, threading)
  - Version history from 0.1.0 to 0.6.0
- **Выгода**: Упрощение работы с UE5 стороны

### IMP-009: CI/CD pipeline
- **Приоритет**: P2
- **Статус**: Выполнено (Сессия 18)
- **Описание**: GitHub Actions для автоматической проверки:
  1. `cargo fmt --check` — форматирование
  2. `cargo clippy` — линтер (lib: `-D warnings`, tests: `-A dead_code`)
  3. `cargo test` — lib + integration + property + edge case тесты
  4. `cargo build --release` — сборка DLL + проверка 46+ exports
- **Файл**: `.github/workflows/ci.yml` (3 jobs: check → test → build-release)
- **Выгода**: Автоматическая проверка перед мержем

### IMP-010: Clippy — включить строгие проверки
- **Приоритет**: P2
- **Статус**: Выполнено (Сессия 17)
- **Описание**: `cargo clippy --lib` → 0 warnings, 0 errors. Исправлено 1 error + 26 warnings в 15 файлах.
- **Выгода**: Чистый код, меньше потенциальных багов

### IMP-011: Разделить engine/mod.rs на подмодули
- **Приоритет**: P3
- **Статус**: ✅ Завершено (Session 19)
- **Описание**: engine/mod.rs — 1242 строки. Разделено на 11 подмодулей:
  - `engine/config.rs` — EngineConfig, TransportMode
  - `engine/services/game_state.rs` — GameStateService
  - `engine/services/combat.rs` — CombatService
  - `engine/services/generation.rs` — GenerationService
  - `engine/services/mastery.rs` — MasteryService
  - `engine/services/economy.rs` — EconomyService
  - `engine/hybrid.rs` — HybridEngine
  - `engine/plugin.rs` — EnginePlugin, Bevy integration
- **Выгода**: Лучшая организация, проще навигация

### IMP-012: AI Asset Pipeline
- **Приоритет**: P2
- **Статус**: Предложено
- **Описание**: Автоматическая генерация 3D-ассетов для монстров:
  - TripoSR / InstantMesh — 3D модели из текста/изображения
  - SDXL / Flux — текстуры из описания
  - AudioCraft / Bark — звуки монстров
  - SAM2 — сегментация для текстур
- **Интеграция**: Rust MonsterGrammar → текстовое описание → AI генерация → Blender обработка → UE5 импорт
- **Выгода**: Уникальные модели для каждого типа монстра

### IMP-013: Добавить tracing crate для логирования
- **Приоритет**: P2
- **Статус**: ✅ Завершено (Session 21)
- **Описание**: Заменить `println!`/`info!` на `tracing` с уровнями и spans
- **Реализация**: `logging/mod.rs` — LogLevel (5 уровней), TracingConfig, LoggingPlugin, idempotent init_tracing()
- **FFI**: logging_get_default_config, logging_init, logging_get_snapshot, logging_log_message
- **UE5**: LoggingConfigWidget.h/.cpp — уровни, модульные фильтры, формат
- **Выгода**:
  - Структурированные логи (JSON формат)
  - Фильтрация по уровням (TRACE/DEBUG/INFO/WARN/ERROR)
  - Spans для отслеживания времени операций
  - Совместимость с Jaeger для distributed tracing

### IMP-014: Hot-reload конфигурации
- **Приоритет**: P3
- **Статус**: ✅ Завершено (Session 22)
- **Описание**: Загружать config/engine.json при старте и поддерживать перезагрузку без перезапуска
- **Реализация**: `hotreload/mod.rs` — file watching via `notify` v6, HotReloadState resource
  - Автоматическая перезагрузка при изменении config/engine.json
  - Валидация JSON перед применением
  - ConfigReloadEvent для Bevy event system
  - Rollback on invalid config
  - 13 unit tests
- **FFI**: hotreload_get_status, hotreload_trigger_reload
- **Выгода**: Быстрая итерация настроек без пересборки

### IMP-015: WebSocket транспорт вместо HTTP polling
- **Приоритет**: P1
- **Статус**: Предложено
- **Описание**: Для real-time обновлений (позиция, комбат) использовать WebSocket вместо HTTP запросов
- **Преимущества**:
  - Меньше overhead (нет HTTP headers на каждый пакет)
  - Двусторонняя связь (push от сервера)
  - Ниже латентность
- **Библиотеки**: `tokio-tungstenite` (Rust), UE5 FWebSocket

### IMP-016: Система миграции для save-файлов
- **Приоритет**: P2
- **Статус**: ✅ Завершено (Session 20)
- **Описание**: При обновлении формата сохранений — система миграции старых → новых
- **Реализация**: `savemigration/mod.rs` — версионирование (v1→v2→v3), MigrationResult, forward-only
- **FFI**: migrate_save, get_save_version, create_new_save, validate_save, get_current_save_version
- **Выгода**: Игроки не теряют прогресс при обновлениях

### IMP-017: Analytics & Telemetry для балансировки
- **Приоритет**: P2
- **Статус**: ✅ Завершено (Session 22)
- **Описание**: Сбор метрик геймплея для балансировки и анализа
- **Реализация**: `analytics/mod.rs` — 5 категорий статистики, 16 типов событий
  - Combat stats: damage dealt/taken, kills by weapon, deaths by tier, parries, dodges
  - Progression stats: floors cleared, highest floor, playtime, secrets found
  - Equipment stats: weapon usage %, socket gems, set bonuses
  - Economy stats: gold earned/spent, items crafted/sold/bought
  - Behavior stats: APM, skill rotation diversity (Shannon entropy), combat duration
  - AnalyticsCollector resource, AnalyticsEvent enum
  - 16 unit tests covering all event types
- **FFI**: analytics_get_snapshot, analytics_reset, analytics_record_damage, analytics_record_floor_cleared, analytics_record_gold, analytics_get_event_types
- **Выгода**: Data-driven балансировка, обнаружение мета-билдов, выявление эксплойтов

---

## Технический долг (TD-001 — TD-XXX)

### TD-001: Дублирование кода в combat/weapons.rs
- **Приоритет**: P3
- **Описание**: Каждый тип оружия повторяет похожую логику инициализации комбо-цепочек
- **Решение**: Создать macro или builder pattern для WeaponData

### TD-002: unwrap() в FFI bridge
- **Приоритет**: P1
- **Статус**: Закрыто (Сессия 17 — аудит: все production FFI безопасны)
- **Описание**: Аудит подтвердил: unwrap() только в тестах. Все 46 production FFI используют match/return null.
- **Решение**: See IMP-007

### TD-003: Тесты не покрывают edge cases
- **Приоритет**: P2
- **Статус**: Выполнено (Сессия 18)
- **Описание**: 110 edge case тестов в `tests/edge_cases.rs`:
  - Null pointer safety (34 теста): каждый FFI с null входом
  - Malformed/empty JSON (16 тестов): невалидный JSON, пустые строки
  - Maximum values (18 тестов): u64::MAX, u32::MAX, f32::MAX
  - Zero/minimum (6 тестов): floor_id=0, 0 XP, 0 count
  - Invalid IDs (10 тестов): unknown action, ability, cosmetic, achievement
  - Season/Social/Replication/Event edge cases (17 тестов)
  - Determinism (2 теста), static getters (1 тест), semantic (3 теста), socket colors (3 теста)
- **Найден и исправлен баг**: integer overflow в `loot/mod.rs` (`20 + floor_level` при u32::MAX)
- **Решение**: proptest (IMP-004) + edge case tests + saturating_add fix

### TD-004: UE5 C++ классы не скомпилированы
- **Приоритет**: P0
- **Описание**: 57 .h + 57 .cpp файлов написаны, но НЕ скомпилированы в UE5 (UE5 не установлен)
- **Риск**: Возможные ошибки компиляции, несовместимости API
- **Решение**: Установить UE5 5.3.2, скомпилировать, исправить ошибки
- **Файл**: TowerGame.uproject + Build.cs

### TD-005: Nakama модули не протестированы
- **Приоритет**: P1
- **Описание**: tower_main.lua + tower_match.lua написаны, но не запускались на реальном Nakama сервере
- **Решение**: `docker-compose up` → тестирование через Nakama Console

### TD-006: Blender скрипты не протестированы
- **Приоритет**: P2
- **Описание**: batch_export.py, validate_models.py, tower_addon.py не тестировались с реальным Blender
- **Решение**: Установить Blender 4.x, протестировать экспорт тестовых моделей

### TD-007: Отсутствует cargo fmt
- **Приоритет**: P3
- **Статус**: Выполнено (Сессия 18)
- **Описание**: `cargo fmt` применён ко всей кодовой базе. CI проверяет `cargo fmt -- --check`
- **Решение**: `cargo fmt` + CI enforcement

### TD-008: Hardcoded constants
- **Приоритет**: P3
- **Статус**: ✅ Завершено (Session 19)
- **Описание**: Многие числовые значения захардкожены и дублируются между engine services и FFI bridge
- **Решение**: Создан `src/constants.rs` — централизованные константы (combat, breath cycle, generation). Обновлены engine/services/ и bridge/mod.rs

---

## Архитектурные улучшения (ARCH-001 — ARCH-XXX)

### ARCH-001: ECS-first подход для engine модуля
- **Приоритет**: P2
- **Описание**: engine/mod.rs использует обычные struct + impl. Лучше использовать Bevy ECS Components/Resources для всех сервисов
- **Выгода**: Параллелизм через Bevy scheduler, query-based доступ к данным

### ARCH-002: Plugin-based архитектура для модулей
- **Приоритет**: P3
- **Описание**: Каждый модуль уже является Bevy Plugin, но не все используют Systems/Schedules оптимально
- **Выгода**: Модули могут быть включены/выключены независимо (feature flags)

### ARCH-003: Отделить game logic от serialization
- **Приоритет**: P2
- **Описание**: Некоторые модули смешивают бизнес-логику с JSON сериализацией
- **Решение**: Чистые game logic функции + отдельный serialization layer (bridge)

### ARCH-004: Использовать Bevy States для game flow
- **Приоритет**: P2
- **Статус**: ✅ Завершено (Session 20)
- **Описание**: Loading → MainMenu → CharacterSelect → InGame → Paused → Death → FloorTransition
- **Реализация**: `gameflow/mod.rs` — GameState (7 вариантов), InGameSubState (7 вариантов), GameFlowPlugin с OnEnter/OnExit
- **FFI**: get_all_game_states, get_all_sub_states
- **Выгода**: Чёткое разделение состояний игры, правильная инициализация/очистка

---

## Предложения по фичам (FEAT-001 — FEAT-XXX)

### FEAT-001: Процедурная музыка
- **Приоритет**: P3
- **Описание**: Генерация музыки на основе семантических тегов этажа
- **Подход**: MIDI генерация из тегов → синтезатор → адаптивный саундтрек
- **Элемент огня → минорная тональность, быстрый темп; элемент воды → мажорная, медленный**

### FEAT-002: Фото-режим
- **Приоритет**: P3
- **Описание**: Пауза + свободная камера + фильтры для скриншотов
- **UE5**: SceneCapture2D с настройками постпроцессинга

### FEAT-003: Replay система
- **Приоритет**: P2
- **Статус**: ✅ Завершено (Session 21)
- **Описание**: Запись и воспроизведение боёв (через DeltaLog)
- **Реализация**: `replay/mod.rs` — InputFrame, ReplayRecording, ReplayPlayback, ReplayRecorder с детерминистичной записью, SHA3 verification, loop support, state machine (Idle/Playing/Paused/Finished/Error)
- **FFI**: replay_start_recording, replay_record_frame, replay_stop_recording, replay_create_playback, replay_get_snapshot, replay_get_input_types
- **UE5**: ReplayControlWidget.h/.cpp — play/pause/stop, speed slider, timeline scrubber, loop toggle

### FEAT-004: Модификаторы этажей (Mutators)
- **Приоритет**: P2
- **Статус**: ✅ Завершено (Session 20)
- **Описание**: 28 типов мутаторов в 5 категориях (Combat, Environment, Economy, Semantic, Challenge)
- **Реализация**: `mutators/mod.rs` — детерминистичная генерация из SHA3(seed+floor_id), tier-based count (1-4), difficulty 1-5, reward scaling +10%/difficulty
- **FFI**: generate_floor_mutators, get_all_mutator_types, compute_mutator_effects
- **UE5**: MutatorWidget.h/.cpp — difficulty stars, category badges, effects summary

### FEAT-005: Карта башни (Tower Map)
- **Приоритет**: P2
- **Статус**: ✅ Завершено (Session 21)
- **Описание**: Глобальная карта всех пройденных этажей с метками прогресса
- **Реализация**: `towermap/mod.rs` — FloorMapEntry (16 properties), TowerMap, completion tracking (weighted: 30% rooms, 40% combat, 20% chests, 10% secrets), MapEvent (8 variants), tier filtering
- **FFI**: towermap_create, towermap_discover_floor, towermap_clear_floor, towermap_record_death, towermap_get_floor, towermap_get_overview, towermap_discover_room, towermap_kill_monster
- **UE5**: TowerMapWidget.h/.cpp — floor list, tier filtering, overview stats, floor detail view

---

## История изменений

| Дата | Изменение |
|------|-----------|
| 2026-02-14 | Создан файл. 16 IMP, 8 TD, 4 ARCH, 5 FEAT |
| 2026-02-14 | Сессия 16: IMP-003 (интеграционные тесты), IMP-005 (бенчмарки), IMP-006 (46 FFI) — выполнены |
| 2026-02-14 | Сессия 17: IMP-004 (proptest), IMP-007/TD-002 (unwrap audit), IMP-010 (clippy) — выполнены |
| 2026-02-14 | Сессия 18: IMP-009 (CI/CD), TD-003 (edge case тесты, 110 шт), TD-007 (cargo fmt) — выполнены. Исправлен integer overflow в loot. |

| 2026-02-14 | Сессия 19: IMP-011 (engine split), TD-008 (constants) — выполнены |
| 2026-02-14 | Сессия 20: FEAT-004 (mutators), ARCH-004 (Bevy States), IMP-016 (save migration) — выполнены. 74 FFI exports, 1021 tests, version 0.4.0 |
| 2026-02-14 | Сессия 21: IMP-013 (tracing/logging), FEAT-003 (replay system), FEAT-005 (tower map) — выполнены. 92 FFI exports (+18), 1137 tests (+116), 3 UE5 widgets, version 0.5.0 |
| 2026-02-14 | Сессия 22: IMP-008 (API docs), IMP-014 (hot-reload), IMP-017 (analytics) — выполнены. 100 FFI exports (+8), 1153 tests (+16), docs/api-reference.md (100 functions), version 0.6.0 |

---

*Последнее обновление: Сессия 22 (2026-02-14)*
*Всего предложений: 17 IMP + 8 TD + 4 ARCH + 5 FEAT = 34*
*Выполнено: IMP-003, IMP-004, IMP-005, IMP-006, IMP-007, IMP-008, IMP-009, IMP-010, IMP-011, IMP-013, IMP-014, IMP-016, IMP-017, TD-002, TD-003, TD-007, TD-008, ARCH-004, FEAT-003, FEAT-004, FEAT-005*

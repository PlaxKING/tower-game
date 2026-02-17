window.BENCHMARK_DATA = {
  "lastUpdate": 1771342628142,
  "repoUrl": "https://github.com/PlaxKING/tower-game",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "56051022+PlaxKING@users.noreply.github.com",
            "name": "PlaxKING",
            "username": "PlaxKING"
          },
          "committer": {
            "email": "56051022+PlaxKING@users.noreply.github.com",
            "name": "PlaxKING",
            "username": "PlaxKING"
          },
          "distinct": true,
          "id": "34c3b21c429c5e7048155792a440aeb6463be921",
          "message": "Fix benchmark push permissions + load test endpoint fields\n\n- Add permissions: contents: write to benchmark job (fixes 403 on\n  gh-pages push)\n- Fix CalculateDamage: target_id -> defender_id, add weapon_id field\n- Fix AddMasteryXP -> TrackProgress with correct field names\n- Load test: 3200 RPS at 100 concurrent, 0% errors on all 11 endpoints\n\nCo-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>",
          "timestamp": "2026-02-17T18:28:13+03:00",
          "tree_id": "908c814e375aa95eac33f52c8d547e8cfd86f3d2",
          "url": "https://github.com/PlaxKING/tower-game/commit/34c3b21c429c5e7048155792a440aeb6463be921"
        },
        "date": 1771342627271,
        "tool": "cargo",
        "benches": [
          {
            "name": "generate_single_floor_10x10",
            "value": 37279,
            "range": "± 628",
            "unit": "ns/iter"
          },
          {
            "name": "generate_single_floor_50x50",
            "value": 37261,
            "range": "± 727",
            "unit": "ns/iter"
          },
          {
            "name": "generate_single_floor_100x100",
            "value": 36979,
            "range": "± 624",
            "unit": "ns/iter"
          },
          {
            "name": "cache_hit",
            "value": 38287,
            "range": "± 642",
            "unit": "ns/iter"
          },
          {
            "name": "cache_miss",
            "value": 293325,
            "range": "± 26044",
            "unit": "ns/iter"
          },
          {
            "name": "parallel_generation/1_workers",
            "value": 125070,
            "range": "± 4159",
            "unit": "ns/iter"
          },
          {
            "name": "parallel_generation/2_workers",
            "value": 125105,
            "range": "± 1026",
            "unit": "ns/iter"
          },
          {
            "name": "parallel_generation/4_workers",
            "value": 125747,
            "range": "± 1169",
            "unit": "ns/iter"
          },
          {
            "name": "parallel_generation/8_workers",
            "value": 126089,
            "range": "± 2588",
            "unit": "ns/iter"
          },
          {
            "name": "warmup_10_floors",
            "value": 166250,
            "range": "± 3555",
            "unit": "ns/iter"
          },
          {
            "name": "warmup_50_floors",
            "value": 720042,
            "range": "± 16214",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}
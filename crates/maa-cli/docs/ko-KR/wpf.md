# WPF GUI 설정 옮기기

`maa migrate wpf`는 MAA Windows GUI가 내보낸 JSON을 maa-cli 작업 파일로 바꿉니다.
지원하지 않는 작업과 설정은 건너뜁니다.
명령이 끝나면 요약을 표준 오류로 씁니다.
매일 실행에 쓰기 전에 결과를 [설정](config.md)과 맞춰 확인합니다.

## 설정 하나 옮기기

입력은 GUI가 내보낸 JSON 파일입니다.

```bash
# 입력과 같은 이름의 .toml을 쓴다
maa migrate wpf gui.json

# 확장자가 형식을 정한다. toml, yaml, json 중 하나
maa migrate wpf gui.json tasks/daily.toml

# 내보내기에 설정이 여러 개이면 이름으로 하나를 고른다
maa migrate wpf gui.json --profile-name Default

# GUI에서 이미 사용자 기지 일정이면, 지정한 일정 파일을 쓴다
maa migrate wpf gui.json --custom-schedule ~/base-schedule.json
```

내보내기에 설정이 하나이면 명령은 그것을 사용합니다.
여러 개이고 `--profile-name`이 없으면 명령이 고르라고 묻습니다.

`--custom-schedule`은 그 기반 시설이 GUI에서 이미 사용자 일정일 때만 동작합니다.
아직 사용자 일정이 아니면 이 옵션을 주면 명령은 오류로 끝납니다.

만든 파일을 `$MAA_CONFIG_DIR/tasks`에 두고 `maa run <task>`로 실행합니다.
설정 디렉터리는 `maa dir config`로 확인합니다.

## 설정 문서에 이미 있는 항목

출력 구조는 [사용자 정의 작업](config.md#사용자-정의-작업)과 같습니다.
맨 위는 `[[tasks]]`입니다.
각 작업에는 `type`, `name`, `params`가 있습니다.
조건으로 바꿀 때는 `variants`도 있습니다.
조건 작성은 [작업 조건](config.md#작업-조건)을 봅니다.

아래에서 말하는 이름은 설정 문서에 이미 있는 이름입니다.
설정 문서에 없고, 실행할 때 MaaCore가 읽는 이름은 다음 절에 있습니다.

### 시작

`client_type`과 `start_game_enabled`는 설정 문서의 시작 예시에 있는 두 항목입니다.
GUI에 알아볼 수 있는 클라이언트가 없으면 `client_type`은 실행 중에 고르는 입력이 됩니다.
작성법은 설정 문서의 사용자 입력을 봅니다.
계정 전환이 켜져 있고 계정 이름이 비어 있으면 그 항목은 쓰지 않습니다.
요약에 건너뛴 필드가 한 줄 나옵니다.

### 이성 작전

칠 스테이지는 각각 변형 하나입니다.
스테이지 이름은 `stage`입니다.

조건은 스테이지 종류를 따릅니다.

- 자원이나 칩 스테이지는 오피셜 서버 시간대의 `Weekday`입니다.
- 이벤트 스테이지는 이 기기의 핫업데이트 이벤트 표에 그 이벤트가 있으면 `DateTime`입니다. 시작 시각, 종료 시각, 시간대가 들어갑니다. 없으면 `Always`가 되고 경고가 납니다.
- 메인, 섬멸, `LS-6` 같은 상설 스테이지는 `Always`입니다.

`DateTime`, `Weekday`, `DayMod`는 설정 문서의 작업 조건에 있습니다.
`Always`도 maa-cli 조건입니다. 시각을 보지 않으므로 그 변형은 항상 실행할 수 있습니다.
이벤트 스테이지의 `start`와 `end`는 옮길 당시 이 기기의 이벤트 표에서 가져옵니다.
이후 핫업데이트에는 따라가지 않습니다.
이벤트가 바뀌면 다시 옮기거나 조건을 직접 고칩니다.
GUI에 주간 계획도 있으면 주간 계획과 스테이지 조건을 `And`로 묶습니다.

아래는 후보 스테이지가 여러 개일 때의 결과입니다.

```toml
[[tasks]]
name = "理智作战"
type = "Fight"

[[tasks.variants]]
condition = { type = "DateTime", start = "2026-08-08T16:00:00", end = "2026-08-22T03:59:59", timezone = 8 }
params = { stage = "TO-5" }

[[tasks.variants]]
condition = { type = "Weekday", weekdays = ["Tue", "Thu", "Sat", "Sun"], timezone = "Official" }
params = { stage = "CE-6" }

[[tasks.variants]]
condition = { type = "Always" }
params = { stage = "LS-6" }
```

결과에 순오리지늄을 쓰면 명령이 경고합니다.
이 설정은 순오리지늄을 쓸 수 있습니다.
이성 물약, 순오리지늄, 횟수, 만료 임박 물약의 매개변수 이름은 설정 문서에 없습니다. 다음 절을 봅니다.

### 기반 시설 교대

시설 순서는 `facility`입니다.
신뢰도가 가득 차지 않은 오퍼레이터를 숙소에 넣을지는 `dorm_trust_enabled`입니다.
이 두 이름은 설정 문서의 기반 시설 예시에 있습니다.

GUI가 시각으로 교대하고 일정의 각 교대조에 시간대가 있으면 `mode`는 `10000`입니다.
변형은 시간대에 따라 `plan_index`를 고릅니다.
자정을 넘는 시간대에는 `DayMod`가 붙습니다.
이것은 설정 문서의 기반 시설 예시와 같은 구성입니다.
일정에 쓸 수 있는 시간대가 없거나 GUI가 고정 교대조를 고르면, 결과는 사용자 기지로 남고 `plan_index`를 포함합니다.

써 넣는 `filename`은 절대 경로입니다.
일정 경로의 `~`는 홈 디렉터리로 펼칩니다.
상대 경로는 명령에 넘긴 JSON이 있는 디렉터리에서 해석합니다.
설정 문서에 있듯이 이 일정 JSON은 maa-cli가 아니라 MaaCore가 읽습니다.
일정 파일 안의 피아메타도 그 파일에 남고, 작업 매개변수로 복사하지 않습니다.

다른 일정을 쓰려면 `--custom-schedule`로 그 JSON 경로를 줍니다.
GUI가 아직 사용자 일정이 아니면 명령은 오류로 끝납니다.
그 파일을 읽지 못해도 오류로 끝나며, 기본 기반 시설로 돌아가지 않습니다.

이 옵션이 없고, 일정이 없거나, 해석할 수 없거나, 교대조가 없으면 `mode`는 `0`으로 씁니다.
요약에는 일정을 건너뛰었다고 나옵니다.

드론, 기분 임계값, 피아메타 명단은 설정 문서에 없는 이름입니다. 다음 절을 봅니다.

## MaaCore에 넘기는 매개변수

[기본 구조](config.md#기본-구조)에 있듯이 작업 `params`의 구체적인 이름은 MAA 통합 문서를 따릅니다.
maa-cli는 그 이름이 맞는지 검사하지 않습니다.
실행할 때 MaaCore가 읽습니다.
아래는 옮길 때 `params`에 쓰지만 설정 문서가 설명하지 않는 이름입니다.
값을 바꿀 때는 이 이름을 바꿉니다.
각 이름의 허용 값은 [MAA 통합 문서][task-types]를 봅니다.

### 실행 여부

`enable`이 `false`이면 그 작업은 실행되지 않습니다.
GUI에서 끈 작업도 파일에는 남고 `enable = false`가 붙습니다.
실행하려면 그 필드를 `true`로 바꾸거나 지웁니다.

### 이성 작전 매개변수

- `medicine`은 이성 물약 개수입니다. 설정 문서는 사용자 입력 예시에만 이 이름이 있고, 이성 물약이라고 쓰지 않습니다.
- `stone`은 순오리지늄 개수입니다.
- `times`는 횟수 상한입니다.
- `series`는 연속 전투 횟수입니다.
- `drops`는 목표 드롭입니다.
- `medicine_expire_days`는 며칠 안에 만료되는 이성 물약을 쓸지입니다. 설정 문서 예시의 `expiring_medicine`은 다른 이름이며, 옮길 때 그 이름을 쓰지 않습니다.
- `DrGrandet`이 `true`이면 순오리지늄 확인 화면에서 이성이 1 회복될 때까지 기다렸다가 바로 사용합니다.

### 공개모집

- `times`는 모집 횟수입니다.
- `refresh`는 3성 태그를 새로고침할지입니다.
- `extra_tags_mode`는 추가 태그를 다루는 방식입니다. 숫자의 의미는 통합 문서를 봅니다.
- `select`와 `confirm`은 자동으로 고르고 자동으로 확정할 성급입니다. 예는 `3`, `4`, `5`입니다.
- `recruitment_time`은 성급별 모집 분입니다.
- `first_tags`는 선호 태그입니다.
- `preserve_tags`는 남길 태그입니다.
- `level3_recruitment_permit_reserve`는 남겨 둘 3성 모집 허가증 수입니다. 이 항목이 꺼져 있거나 수량이 0보다 크지 않으면 이 이름을 쓰지 않습니다.

GUI의 즉시 새로고침은 쓰지 않습니다.
켜져 있으면 요약에 건너뛴 필드가 한 줄 나오고 경고도 납니다.
6성 자동 확정이 켜져 있으면 명령이 경고합니다.

### 기반 시설 교대 매개변수

- `drones`는 드론 용도입니다.
- `threshold`는 숙소 기분 임계값이며 범위는 0에서 1입니다. GUI의 백분율은 이 소수가 됩니다.
- `replenish`는 무역소의 오리지늄 파편을 자동 보충할지입니다.
- `dorm_notstationed_enabled`는 숙소에서 「미배치」를 쓸지입니다.
- `reception_message_board`, `reception_clue_exchange`, `reception_send_clue`는 응접실 메시지 보드, 단서 교환, 단서 보내기입니다.
- `continue_training`은 훈련실에서 훈련을 계속할지입니다.
- `fiammetta_targets`는 피아메타가 기분을 회복할 오퍼레이터 명단입니다.
- `fiammetta_recovery_enabled`는 피아메타로 기분을 회복할지입니다.
- `use_pinus_sylvestris`, `use_perception_information`, `use_worldly_plight`, `use_abyssal_hunter`는 홍송, 지각, 진세, 심해 네 가지 시설 조합입니다. `false`이면 쓰지 않습니다.

`fiammetta_targets`는 `mode`가 0이고 `fiammetta_recovery_enabled`가 true일 때만 적용됩니다.
`fiammetta_recovery_enabled`와 위 네 조합은 `mode`가 0일 때만 적용됩니다.

### 크레딧 상점

- `shopping`은 크레딧 상품을 살지입니다.
- `credit_fight`는 크레딧 전투를 할지입니다.
- `formation_index`는 크레딧 전투에 쓸 편성입니다.
- `visit_friends`는 친구를 방문할지입니다.
- `buy_first`는 먼저 살 상품입니다.
- `blacklist`는 사지 않을 상품입니다.
- `force_shopping_if_credit_full`은 크레딧이 가득 찼을 때 차단 목록 밖을 아직 살지입니다.
- `only_buy_discount`는 할인 상품만 살지입니다.
- `reserve_max_credit`은 크레딧을 상한 근처에 남길지입니다.

크레딧 전투와 친구 방문의 마지막 시각, 오늘 아직 할 수 있는지는 GUI 자신의 기록입니다.
결과에는 쓰지 않습니다.

### 보상

- `award`는 일일 또는 주간 임무 보상을 받습니다.
- `mail`은 우편 보상을 받습니다.
- `recruit`는 한정 모집에서 주는 매일 무료 1회를 받습니다.
- `orundum`은 행운의 벽 합성옥을 받습니다.
- `mining`은 기간 한정 채굴 허가증의 합성옥을 받습니다.
- `specialaccess`는 5주년 월간권을 받습니다.
- `signinevent`는 기간 한정 출석 보상을 받습니다.

### 로그라이크

이 설정은 실기에서 확인하지 않았습니다.
각 매개변수의 작용은 [MAA 통합 문서][task-types]를 봅니다.

이 이름들은 해당하는 테마와 모드에서만 의미가 있습니다.
GUI에서 고른 내용을 남기려고 씁니다.
쓰지 않는 모드에서 MaaCore는 이들을 무시합니다.

- `theme`는 테마입니다.
- `mode`는 모드 번호입니다. 점수 파밍은 `0`, 원석정 파밍은 `1`, 시작 보상 맞추기는 `4`, 붕괴 패러다임은 `5`, 월간 분대는 `6`, 심층 조사는 `7`, 상락 노드 찾기는 `20001`, 흑류수해 시작 런은 `30003`입니다. 알아보지 못한 모드는 `mode`를 쓰지 않고 요약에 기록합니다.
- `squad`는 시작 분대입니다. `roles`는 시작 직업 조합입니다.
- `core_char`는 첫 시작 오퍼레이터입니다. `core_char_list`는 시작 오퍼레이터 전체이며 각 항목에 `name`과 `use_support`가 있습니다.
- `use_support`는 첫 오퍼레이터를 서포트로 쓸지입니다. `use_nonfriend_support`는 친구가 아닌 서포트를 쓸지입니다.
- `starts_count`는 시작 횟수입니다. `difficulty`는 난이도입니다. 난이도가 자리 표시용인 매우 큰 수이면 쓰지 않습니다.
- `investment_enabled`는 투자할지입니다. `investments_count`는 투자 횟수입니다. `investment_with_more_score`는 더 높은 점수로 투자할지입니다. `stop_when_investment_full`은 예금이 가득 차면 멈출지입니다.
- `stop_at_final_boss`는 엔딩 전에 멈출지입니다. `stop_at_max_level`은 레벨이 최대이면 멈출지입니다.
- `start_with_elite_two`와 `only_start_with_elite_two`는 정예 2 시작입니다.
- `refresh_trader_with_dice`는 주사위로 상점을 새로고침할지입니다.
- `collectible_mode_squad`는 시작 보상 맞추기 또는 흑류수해 준비 판에서 쓰는 분대입니다. `collectible_mode_shopping`은 시작 보상 맞추기에서 쇼핑할지입니다. `collectible_mode_start_list`는 원하는 시작 보상이며 예는 `hot_water`입니다.
- `monthly_squad_auto_iterate`는 월간 분대를 자동으로 바꿀지입니다. `monthly_squad_check_comms`는 바꿀 때 통신을 볼지입니다.
- `deep_exploration_auto_iterate`는 심층 조사를 자동으로 바꿀지입니다.
- `find_playTime_target`은 상락 노드입니다. `1`은 링, `2`는 슈, `3`은 니엔입니다.
- `blackflow_strategy`는 흑류수해 엔딩 전략이며 예는 `ending1`입니다. `blackflow_cultivation_target`은 키울 강보 동물입니다. `blackflow_start_animals`는 준비 판에서 모을 강보 동물입니다.
- `expected_collapsal_paradigms`는 원하는 붕괴 패러다임입니다.
- `first_floor_foldartal`과 `start_foldartal_list`는 밀문판입니다.
- `start_with_seed`는 고정 시드입니다.

흑류수해 시작 보상의 비트 조합과 빠른 비행은 쓰지 않습니다.
GUI에서 이 둘이 꺼짐이나 「없음」이 아니면 요약에 건너뛴 필드가 각각 한 줄 나옵니다.

### 생존 연산

이 설정은 실기에서 확인하지 않았습니다.
각 매개변수의 작용은 [MAA 통합 문서][task-types]를 봅니다.

- `theme`는 테마입니다.
- `mode` `0`은 저장하지 않는 번영, `1`은 세이브에 쓰는 번영입니다. 알아보지 못한 모드는 쓰지 않고 요약에 기록합니다.
- `tools_to_craft`는 만들 도구입니다.
- `increment_mode`는 증분 모드입니다.
- `num_craft_batches`는 한 라운드의 제작 배치 수입니다.

창고 인식과 오퍼레이터 box는 작업이 되지 않습니다.
알 수 없는 작업도 작업이 되지 않습니다.
둘 다 요약에만 나옵니다.

## 요약

건너뛰거나 끈 작업이 있으면 요약은 표준 오류로 갑니다.
잃은 것이 없으면 아무것도 인쇄하지 않습니다.
명령이 인쇄하는 글은 영어입니다.
아래 세 항목이 항상 함께 나오지는 않습니다.

```text
Migration summary (lossy):
  Skipped tasks (unsupported type):
    - UserDataUpdateTask "同步数据"
  Disabled tasks (kept with params.enable=false; will not run):
    - FightTask "剿灭"
  Skipped fields (not migrated):
    - RecruitTask "公开招募": ForceRefresh
```

첫 줄은 이번 변환이 일부를 버렸다는 뜻입니다.

`Skipped tasks`는 파일에 아예 쓰지 않은 작업입니다.
따옴표 안은 GUI에 있는 그 작업의 이름입니다.
`UserDataUpdateTask`는 창고 인식과 오퍼레이터 box입니다.

`Disabled tasks`는 파일에는 썼지만 기본적으로 실행하지 않는 작업입니다.
괄호 안의 `params.enable=false`가 앞 절의 `enable`입니다.
따옴표 안은 작업 이름입니다.
`FightTask`는 작전, `RecruitTask`는 공개모집, `InfrastTask`는 기반 시설입니다.

`Skipped fields`는 어떤 작업 안에서 쓰지 않은 설정 하나입니다.
따옴표 안은 작업 이름입니다.
콜론 뒤는 GUI 내보내기 파일에 있던 원래 필드 이름입니다.
그 이름을 알 필요는 없습니다.
그 설정이 버려졌다는 뜻만 있습니다.
공개모집에서 즉시 새로고침이 켜져 있으면 위 마지막 줄이 됩니다.

매일 실행에 쓰기 전에 요약을 읽습니다.

## 참고

- [설정](config.md)
- [사용법](usage.md)
- [MAA 통합 문서의 작업 유형][task-types]

[task-types]: https://maa.plus/docs/ko-kr/protocol/integration.html
